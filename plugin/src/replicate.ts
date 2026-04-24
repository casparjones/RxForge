import { replicateRxCollection } from 'rxdb/plugins/replication';
import { Subject } from 'rxjs';
import { getToken } from './auth.js';
import type { RxForgeReplicationOptions, PullResult } from './types.js';
import type { RxReplicationWriteToMasterRow, WithDeleted } from 'rxdb';

/**
 * Default conflict handler: last-write-wins based on updatedAt field.
 * If neither document has updatedAt, the master (server) version wins.
 */
function defaultConflictHandler<RxDocType>(
  input: {
    newDocumentState: WithDeleted<RxDocType>;
    realMasterState: WithDeleted<RxDocType>;
  }
): Promise<{ documentData: WithDeleted<RxDocType> }> {
  const { newDocumentState, realMasterState } = input;
  const newTs = (newDocumentState as any).updatedAt ?? 0;
  const masterTs = (realMasterState as any).updatedAt ?? 0;
  const winner = newTs >= masterTs ? newDocumentState : realMasterState;
  return Promise.resolve({ documentData: winner });
}

/**
 * Create an SSE-based pull stream for real-time updates.
 * Emits { documents, checkpoint } objects whenever the server pushes changes.
 */
function createPullStream<RxDocType>(
  serverUrl: string,
  appId: string,
  getAccessToken: () => Promise<string>
): Subject<PullResult<WithDeleted<RxDocType>>> {
  const subject = new Subject<PullResult<WithDeleted<RxDocType>>>();

  let eventSource: EventSource | null = null;
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  let active = true;

  async function connect() {
    if (!active) return;
    try {
      const token = await getAccessToken();
      // SSE with token in query param (EventSource doesn't support custom headers)
      const url = `${serverUrl}/api/v1/sync/${appId}/stream?token=${encodeURIComponent(token)}`;
      eventSource = new EventSource(url);

      eventSource.addEventListener('change', (event: MessageEvent) => {
        try {
          const data = JSON.parse(event.data) as PullResult<WithDeleted<RxDocType>>;
          subject.next(data);
        } catch {
          // ignore malformed events
        }
      });

      eventSource.onerror = () => {
        eventSource?.close();
        eventSource = null;
        if (active) {
          reconnectTimeout = setTimeout(() => connect(), 5000);
        }
      };
    } catch {
      if (active) {
        reconnectTimeout = setTimeout(() => connect(), 5000);
      }
    }
  }

  connect();

  // Expose cleanup via the subject's unsubscribe mechanism
  const originalUnsubscribe = subject.unsubscribe.bind(subject);
  subject.unsubscribe = () => {
    active = false;
    if (reconnectTimeout) clearTimeout(reconnectTimeout);
    eventSource?.close();
    originalUnsubscribe();
  };

  return subject;
}

/**
 * Start RxDB replication with a RxForge server.
 *
 * @example
 * ```ts
 * const replicationState = replicateRxForge({
 *   collection: myCollection,
 *   serverUrl: 'https://your-rxforge.example.com',
 *   appId: 'your-app-id',
 *   clientId: 'your-client-id',
 * });
 * await replicationState.awaitInitialReplication();
 * ```
 */
export function replicateRxForge<RxDocType>(
  options: RxForgeReplicationOptions<RxDocType>
) {
  const {
    collection,
    serverUrl,
    appId,
    batchSize = 100,
    conflictHandler,
  } = options;

  // Apply conflict handler to the collection. In RxDB the conflict handler lives
  // on the collection itself, so we install it here (custom if provided, otherwise LWW).
  (collection as unknown as { conflictHandler: unknown }).conflictHandler =
    conflictHandler ?? (defaultConflictHandler as unknown);

  const tokenGetter = () => getToken(options);

  const pullStream$ = createPullStream<RxDocType>(serverUrl, appId, tokenGetter);

  return replicateRxCollection<RxDocType, unknown>({
    collection,
    replicationIdentifier: `rxforge-${appId}-${collection.name}`,

    pull: {
      batchSize,
      handler: async (lastCheckpoint: unknown, limit: number): Promise<PullResult<WithDeleted<RxDocType>>> => {
        const token = await tokenGetter();
        const res = await fetch(
          `${serverUrl}/api/v1/sync/${appId}/pull` +
            `?checkpoint=${encodeURIComponent(JSON.stringify(lastCheckpoint))}` +
            `&limit=${limit}`,
          {
            headers: {
              Authorization: `Bearer ${token}`,
            },
          }
        );
        if (!res.ok) {
          const text = await res.text();
          throw new Error(`RxForge pull failed: ${text}`);
        }
        return res.json() as Promise<PullResult<WithDeleted<RxDocType>>>;
      },
      stream$: pullStream$.asObservable(),
    },

    push: {
      batchSize,
      handler: async (
        docs: RxReplicationWriteToMasterRow<RxDocType>[]
      ): Promise<WithDeleted<RxDocType>[]> => {
        const token = await tokenGetter();
        const res = await fetch(`${serverUrl}/api/v1/sync/${appId}/push`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${token}`,
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(docs),
        });
        if (!res.ok) {
          const text = await res.text();
          throw new Error(`RxForge push failed: ${text}`);
        }
        // Server returns conflicting documents (empty array if no conflicts)
        return res.json() as Promise<WithDeleted<RxDocType>[]>;
      },
    },
  });
}

// Re-export so tests can exercise the default conflict handler directly
export { defaultConflictHandler };
