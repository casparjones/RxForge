/**
 * RxForge sync integration for RxDB 15+
 *
 * Uses RxDB's generic replication protocol (replicateRxCollection), NOT replicateCouchDB.
 * Documents use `id` as primary key; no `_rev` fields.
 * Soft-deletes use `_deleted: true`.
 */

import {
  createRxDatabase,
  addRxPlugin,
  type RxDatabase,
  type RxCollection,
  type RxConflictHandler,
  type RxConflictHandlerInput,
  type WithDeleted,
} from 'rxdb';
import { getRxStorageDexie } from 'rxdb/plugins/storage-dexie';
import { replicateRxCollection, type RxReplicationState } from 'rxdb/plugins/replication';
import { Subject } from 'rxjs';

// ─── Schema ──────────────────────────────────────────────────────────────────

interface TodoDoc {
  id: string;
  text: string;
  done: boolean;
  updatedAt: number;
}

const todoSchema = {
  title: 'todo schema',
  version: 0,
  primaryKey: 'id',
  type: 'object',
  properties: {
    id:        { type: 'string', maxLength: 100 },
    text:      { type: 'string' },
    done:      { type: 'boolean' },
    updatedAt: { type: 'number', minimum: 0 },
  },
  required: ['id', 'text', 'done', 'updatedAt'],
} as const;

// ─── Config ───────────────────────────────────────────────────────────────────

const APP_ID   = 'f5329f79-12ef-4bd9-88ea-045968ba3706';
const BASE_URL = 'http://localhost:8080';
const SYNC_URL = `${BASE_URL}/api/v1/sync/${APP_ID}`;

// ─── JWT cache ────────────────────────────────────────────────────────────────

interface JwtEntry { token: string; expiresAtMs: number }
let _jwt: JwtEntry | null = null;

/**
 * Exchange an `rxft_…` public token for a short-lived write JWT, caching the
 * result and refreshing automatically when fewer than 2 minutes remain.
 */
async function getJwt(rxftToken: string): Promise<string> {
  const now = Date.now();
  if (_jwt && _jwt.expiresAtMs - now > 2 * 60_000) return _jwt.token;

  const res = await fetch(`${BASE_URL}/api/v1/auth/token/exchange`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token: rxftToken }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.status.toString());
    throw new Error(`Token exchange failed (${res.status}): ${text}`);
  }
  const data = await res.json() as { access_token: string; expires_in: number };
  _jwt = { token: data.access_token, expiresAtMs: now + data.expires_in * 1000 };
  return _jwt.token;
}

// ─── Database singleton ───────────────────────────────────────────────────────

type AppDb = RxDatabase<{ todos: RxCollection<TodoDoc> }>;
let _db: AppDb | null = null;

async function getOrCreateDb(): Promise<AppDb> {
  if (_db) return _db;
  _db = await createRxDatabase<{ todos: RxCollection<TodoDoc> }>({
    name: 'rxforge_kcalc',
    storage: getRxStorageDexie(),
  });
  await _db.addCollections({ todos: { schema: todoSchema } });
  return _db;
}

// ─── Conflict handler — last write wins ───────────────────────────────────────

const conflictHandler: RxConflictHandler<TodoDoc> = async (
  input: RxConflictHandlerInput<TodoDoc>
): Promise<{ documentData: WithDeleted<TodoDoc> }> => {
  const { newDocumentState, realMasterState } = input;
  return {
    documentData:
      newDocumentState.updatedAt >= realMasterState.updatedAt
        ? newDocumentState
        : realMasterState,
  };
};

// ─── SSE stream helper ────────────────────────────────────────────────────────

type PullEnvelope = { documents: WithDeleted<TodoDoc>[]; checkpoint: string };

function createLiveStream(
  rxftToken: string,
  subject: Subject<PullEnvelope>
): () => void {
  let abortCtrl: AbortController | null = null;
  let delay = 1_000;
  let stopped = false;

  async function connect() {
    if (stopped) return;
    abortCtrl?.abort();
    abortCtrl = new AbortController();

    try {
      const res = await fetch(
        `${SYNC_URL}/stream?token=${encodeURIComponent(rxftToken)}`,
        { signal: abortCtrl.signal }
      );
      if (!res.ok || !res.body) throw new Error(`SSE ${res.status}`);
      delay = 1_000; // reset backoff on success

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = '';
      let eventType = '';
      const dataLines: string[] = [];

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const lines = buf.split('\n');
        buf = lines.pop() ?? '';

        for (const line of lines) {
          if (line.startsWith('event: ')) {
            eventType = line.slice(7).trim();
          } else if (line.startsWith('data: ')) {
            dataLines.push(line.slice(6));
          } else if (line === '') {
            if (eventType === 'change' && dataLines.length) {
              try {
                const payload = JSON.parse(dataLines.join('\n')) as {
                  documents: any[];
                  checkpoint: unknown;
                };
                subject.next({
                  documents: (payload.documents ?? []).map(stripInternal) as WithDeleted<TodoDoc>[],
                  checkpoint: String(payload.checkpoint ?? ''),
                });
              } catch { /* malformed event — ignore */ }
            }
            eventType = '';
            dataLines.length = 0;
          }
        }
      }
    } catch (err: unknown) {
      if ((err as { name?: string })?.name === 'AbortError') return;
    }

    // Reconnect with exponential backoff (max 30 s)
    if (!stopped) {
      setTimeout(connect, delay);
      delay = Math.min(delay * 2, 30_000);
    }
  }

  connect();
  return () => { stopped = true; abortCtrl?.abort(); };
}

// ─── Strip storage-internal fields ───────────────────────────────────────────

function stripInternal<T extends Record<string, unknown>>(doc: T): T {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const { _id, _rev, _seq, ...rest } = doc as Record<string, unknown>;
  return rest as T;
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Initialise RxDB (idempotent) and start live replication with RxForge.
 *
 * @param rxftToken  The `rxft_…` public API token from your RxForge app dashboard.
 *                   Used directly for reads; exchanged for a short-lived JWT for writes.
 * @returns          The RxReplicationState — call `.cancel()` to stop sync and listen
 *                   to `.error$` for errors.
 *
 * @example
 * ```ts
 * const replication = await startSync('rxft_abc123…');
 * await replication.awaitInitialReplication();
 * // db is ready
 * replication.cancel(); // stop when done
 * ```
 */
export async function startSync(
  rxftToken: string
): Promise<RxReplicationState<TodoDoc, string>> {
  const db = await getOrCreateDb();
  const collection = db.todos;

  const stream$ = new Subject<PullEnvelope>();
  const stopStream = createLiveStream(rxftToken, stream$);

  const replication = replicateRxCollection<TodoDoc, string>({
    collection,
    replicationIdentifier: `rxforge-${APP_ID}-todos-v1`,
    deletedField: '_deleted',
    live: true,
    retryTime: 5_000,

    pull: {
      batchSize: 100,
      stream$,
      async handler(lastCheckpoint, batchSize) {
        const qs = new URLSearchParams({ limit: String(batchSize) });
        if (lastCheckpoint) qs.set('checkpoint', lastCheckpoint);

        const res = await fetch(`${SYNC_URL}/pull?${qs}`, {
          headers: { Authorization: `Bearer ${rxftToken}` },
        });
        if (!res.ok) throw new Error(`Pull failed (${res.status})`);

        const body = await res.json() as { documents: any[]; checkpoint: unknown };
        return {
          documents: (body.documents ?? []).map(stripInternal) as WithDeleted<TodoDoc>[],
          checkpoint: String(body.checkpoint ?? ''),
        };
      },
    },

    push: {
      batchSize: 50,
      async handler(rows) {
        const jwt = await getJwt(rxftToken);
        const res = await fetch(`${SYNC_URL}/push`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${jwt}`,
          },
          body: JSON.stringify({ rows }),
        });
        if (!res.ok) throw new Error(`Push failed (${res.status})`);

        const body = await res.json() as { conflicts: any[] };
        // Return full conflict documents; RxDB feeds them to conflictHandler
        return (body.conflicts ?? []).map(stripInternal) as WithDeleted<TodoDoc>[];
      },
    },

    conflictHandler,
  });

  // Clean up SSE when replication is cancelled
  replication.cancel$.subscribe(() => stopStream());

  return replication;
}
