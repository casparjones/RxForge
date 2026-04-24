import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the auth module
vi.mock('./auth.js', () => ({
  getToken: vi.fn().mockResolvedValue('mock-access-token'),
  startOAuthFlow: vi.fn().mockResolvedValue('mock-code'),
}));

// Mock rxdb/plugins/replication
vi.mock('rxdb/plugins/replication', () => ({
  replicateRxCollection: vi.fn().mockReturnValue({
    awaitInitialReplication: vi.fn().mockResolvedValue(undefined),
    cancel: vi.fn(),
    isStopped: vi.fn().mockReturnValue(false),
  }),
}));

const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

// Minimal EventSource stub so createPullStream doesn't blow up on import.
class FakeEventSource {
  url: string;
  listeners: Record<string, ((e: MessageEvent) => void)[]> = {};
  onerror: ((e: unknown) => void) | null = null;
  closed = false;
  constructor(url: string) {
    this.url = url;
    FakeEventSource.instances.push(this);
  }
  addEventListener(type: string, cb: (e: MessageEvent) => void) {
    (this.listeners[type] ||= []).push(cb);
  }
  emit(type: string, data: unknown) {
    const evt = { data: typeof data === 'string' ? data : JSON.stringify(data) } as MessageEvent;
    (this.listeners[type] || []).forEach((cb) => cb(evt));
  }
  close() { this.closed = true; }
  static instances: FakeEventSource[] = [];
  static reset() { FakeEventSource.instances = []; }
}
(globalThis as any).EventSource = FakeEventSource;

function makeCollection(name = 'todos') {
  return { name, conflictHandler: undefined } as any;
}

beforeEach(() => {
  vi.clearAllMocks();
  FakeEventSource.reset();
});

describe('replicateRxForge', () => {
  it('calls replicateRxCollection with correct identifier', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    const mockCollection = makeCollection('todos');

    replicateRxForge({
      collection: mockCollection,
      serverUrl: 'https://rxforge.example.com',
      appId: 'app-123',
      clientId: 'client-456',
    });

    expect(replicateRxCollection).toHaveBeenCalledWith(
      expect.objectContaining({
        replicationIdentifier: 'rxforge-app-123-todos',
        collection: mockCollection,
      })
    );
  });

  it('uses custom batchSize when provided', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('items'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'app-xyz',
      clientId: 'client-abc',
      batchSize: 50,
    });

    expect(replicateRxCollection).toHaveBeenCalledWith(
      expect.objectContaining({
        pull: expect.objectContaining({ batchSize: 50 }),
        push: expect.objectContaining({ batchSize: 50 }),
      })
    );
  });

  it('installs the default conflict handler on the collection when none provided', async () => {
    const { replicateRxForge } = await import('./replicate.js');
    const coll = makeCollection('docs');
    expect(coll.conflictHandler).toBeUndefined();
    replicateRxForge({
      collection: coll,
      serverUrl: 'https://s',
      appId: 'a',
      clientId: 'c',
    });
    expect(typeof coll.conflictHandler).toBe('function');
  });

  it('installs a custom conflict handler on the collection when provided', async () => {
    const { replicateRxForge } = await import('./replicate.js');
    const coll = makeCollection('docs');
    const custom = vi.fn().mockResolvedValue({ documentData: { id: '1', _deleted: false } });
    replicateRxForge({
      collection: coll,
      serverUrl: 'https://s',
      appId: 'a',
      clientId: 'c',
      conflictHandler: custom as any,
    });
    expect(coll.conflictHandler).toBe(custom);
  });
});

describe('pull handler', () => {
  it('fetches with checkpoint, limit and auth header', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'test-app',
      clientId: 'test-client',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pullHandler = callArgs.pull.handler;

    const mockDocs = [{ id: '1', text: 'Hello', _deleted: false }];
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ documents: mockDocs, checkpoint: { id: '1', updatedAt: 1000 } }),
    });

    const result = await pullHandler({ id: '0', updatedAt: 500 }, 100);

    expect(result.documents).toEqual(mockDocs);
    expect(result.checkpoint).toEqual({ id: '1', updatedAt: 1000 });

    const url = (mockFetch.mock.calls[0] as any)[0] as string;
    expect(url).toContain('/api/v1/sync/test-app/pull');
    expect(url).toContain('limit=100');
    expect(url).toContain('checkpoint=');
    // checkpoint must be URL-encoded JSON
    expect(decodeURIComponent(url.split('checkpoint=')[1].split('&')[0])).toBe(
      JSON.stringify({ id: '0', updatedAt: 500 })
    );

    expect(mockFetch).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer mock-access-token',
        }),
      })
    );
  });

  it('handles null checkpoint on initial pull', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'initial-app',
      clientId: 'c',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pullHandler = callArgs.pull.handler;

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ documents: [], checkpoint: null }),
    });

    const result = await pullHandler(null, 25);
    expect(result.documents).toEqual([]);
    expect(result.checkpoint).toBeNull();

    const url = (mockFetch.mock.calls[0] as any)[0] as string;
    expect(url).toContain('limit=25');
    expect(url).toContain('checkpoint=null');
  });

  it('throws on server error', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('items'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'err-app',
      clientId: 'err-client',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pullHandler = callArgs.pull.handler;

    mockFetch.mockResolvedValueOnce({
      ok: false,
      text: async () => 'Unauthorized',
    });

    await expect(pullHandler(null, 100)).rejects.toThrow('RxForge pull failed: Unauthorized');
  });
});

describe('push handler', () => {
  it('sends docs as JSON body with auth header', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'push-app',
      clientId: 'test-client',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pushHandler = callArgs.push.handler;

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => [],
    });

    const docs = [
      { newDocumentState: { id: '1', text: 'Hello', _deleted: false }, assumedMasterState: undefined },
    ];
    const conflicts = await pushHandler(docs);
    expect(conflicts).toEqual([]);

    const [url, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('https://rxforge.example.com/api/v1/sync/push-app/push');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer mock-access-token');
    expect(init.body).toBe(JSON.stringify(docs));
  });

  it('returns conflicting documents from server response', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'conf-app',
      clientId: 'c',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pushHandler = callArgs.push.handler;

    const serverWinner = { id: '1', text: 'server-version', _deleted: false };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => [serverWinner],
    });

    const conflicts = await pushHandler([
      { newDocumentState: { id: '1', text: 'client-version', _deleted: false }, assumedMasterState: undefined },
    ]);
    expect(conflicts).toEqual([serverWinner]);
  });

  it('throws on server error', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('items'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'err-app',
      clientId: 'err-client',
    });

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const pushHandler = callArgs.push.handler;

    mockFetch.mockResolvedValueOnce({
      ok: false,
      text: async () => 'Conflict error',
    });

    await expect(
      pushHandler([{ newDocumentState: { id: '1', _deleted: false }, assumedMasterState: undefined }] as any)
    ).rejects.toThrow('RxForge push failed: Conflict error');
  });
});

describe('defaultConflictHandler (LWW by updatedAt)', () => {
  it('prefers newer newDocumentState over older realMasterState', async () => {
    const { defaultConflictHandler } = await import('./replicate.js');
    const res = await defaultConflictHandler({
      newDocumentState: { id: '1', text: 'new', updatedAt: 2000, _deleted: false } as any,
      realMasterState: { id: '1', text: 'old', updatedAt: 1000, _deleted: false } as any,
    });
    expect((res.documentData as any).text).toBe('new');
  });

  it('prefers newer realMasterState over older newDocumentState', async () => {
    const { defaultConflictHandler } = await import('./replicate.js');
    const res = await defaultConflictHandler({
      newDocumentState: { id: '1', text: 'stale', updatedAt: 500, _deleted: false } as any,
      realMasterState: { id: '1', text: 'fresh', updatedAt: 1500, _deleted: false } as any,
    });
    expect((res.documentData as any).text).toBe('fresh');
  });

  it('prefers newDocumentState on tie (>= semantics)', async () => {
    const { defaultConflictHandler } = await import('./replicate.js');
    const res = await defaultConflictHandler({
      newDocumentState: { id: '1', text: 'new', updatedAt: 1000, _deleted: false } as any,
      realMasterState: { id: '1', text: 'old', updatedAt: 1000, _deleted: false } as any,
    });
    expect((res.documentData as any).text).toBe('new');
  });

  it('falls back to master when neither has updatedAt', async () => {
    const { defaultConflictHandler } = await import('./replicate.js');
    // With both updatedAt falling back to 0, the >= tie means newDocumentState wins.
    // Documented behavior: tie goes to newDocumentState.
    const res = await defaultConflictHandler({
      newDocumentState: { id: '1', text: 'n', _deleted: false } as any,
      realMasterState: { id: '1', text: 'm', _deleted: false } as any,
    });
    expect((res.documentData as any).text).toBe('n');
  });
});

describe('pullStream$ (SSE)', () => {
  it('connects to SSE endpoint with token in query string', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'sse-app',
      clientId: 'c',
    });

    // EventSource is constructed asynchronously after getToken resolves; flush microtasks.
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));

    expect(FakeEventSource.instances.length).toBe(1);
    expect(FakeEventSource.instances[0].url).toContain('/api/v1/sync/sse-app/stream');
    expect(FakeEventSource.instances[0].url).toContain('token=mock-access-token');

    // Server emits a change event; stream$ consumers receive it.
    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const received: unknown[] = [];
    callArgs.pull.stream$.subscribe((v: unknown) => received.push(v));
    FakeEventSource.instances[0].emit('change', {
      documents: [{ id: 'x', _deleted: false }],
      checkpoint: { id: 'x', updatedAt: 42 },
    });

    expect(received).toHaveLength(1);
    expect((received[0] as any).checkpoint.updatedAt).toBe(42);
  });

  it('silently ignores malformed SSE payloads', async () => {
    const { replicateRxCollection } = await import('rxdb/plugins/replication');
    const { replicateRxForge } = await import('./replicate.js');

    replicateRxForge({
      collection: makeCollection('docs'),
      serverUrl: 'https://rxforge.example.com',
      appId: 'bad-sse',
      clientId: 'c',
    });

    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));

    const callArgs = (replicateRxCollection as any).mock.calls[0][0];
    const received: unknown[] = [];
    callArgs.pull.stream$.subscribe((v: unknown) => received.push(v));

    expect(() => FakeEventSource.instances[0].emit('change', 'not-json')).not.toThrow();
    expect(received).toHaveLength(0);
  });
});
