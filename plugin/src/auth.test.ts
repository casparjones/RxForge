import { describe, it, expect, vi, beforeEach } from 'vitest';
import { exchangeCodeForToken, refreshAccessToken } from './auth.js';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock, writable: true });

// Minimal window stub so getToken can compute a default redirectUri when needed.
Object.defineProperty(globalThis, 'window', {
  value: {
    location: { origin: 'https://myapp.example' },
    addEventListener: () => {},
    removeEventListener: () => {},
    open: () => null,
  },
  writable: true,
});

const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

beforeEach(() => {
  localStorageMock.clear();
  vi.clearAllMocks();
});

describe('exchangeCodeForToken', () => {
  it('exchanges authorization code for tokens successfully', async () => {
    const mockResponse = {
      access_token: 'access-abc',
      refresh_token: 'refresh-xyz',
      expires_in: 3600,
      token_type: 'Bearer',
    };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => mockResponse,
    });

    const result = await exchangeCodeForToken(
      'https://rxforge.example.com',
      'client-123',
      'auth-code-456',
      'https://myapp.com/callback'
    );

    expect(result.access_token).toBe('access-abc');
    expect(result.refresh_token).toBe('refresh-xyz');
    expect(result.expires_in).toBe(3600);

    expect(mockFetch).toHaveBeenCalledWith(
      'https://rxforge.example.com/oauth/token',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      })
    );
  });

  it('sends grant_type=authorization_code with code and redirect_uri', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        access_token: 'a',
        refresh_token: 'r',
        expires_in: 60,
        token_type: 'Bearer',
      }),
    });

    await exchangeCodeForToken('https://s.example', 'cid', 'thecode', 'https://app/cb');

    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    const body = init.body as string;
    expect(body).toContain('grant_type=authorization_code');
    expect(body).toContain('code=thecode');
    expect(body).toContain('client_id=cid');
    // redirect_uri is URL-encoded
    expect(body).toContain('redirect_uri=https%3A%2F%2Fapp%2Fcb');
  });

  it('throws when server returns error', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      text: async () => 'invalid_grant',
    });

    await expect(
      exchangeCodeForToken('https://rxforge.example.com', 'client-123', 'bad-code', 'https://myapp.com/callback')
    ).rejects.toThrow('Token exchange failed: invalid_grant');
  });
});

describe('refreshAccessToken', () => {
  it('refreshes an expired token successfully', async () => {
    const mockResponse = {
      access_token: 'new-access-token',
      refresh_token: 'new-refresh-token',
      expires_in: 3600,
      token_type: 'Bearer',
    };
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => mockResponse,
    });

    const result = await refreshAccessToken(
      'https://rxforge.example.com',
      'client-123',
      'old-refresh-token'
    );

    expect(result.access_token).toBe('new-access-token');
    expect(result.refresh_token).toBe('new-refresh-token');

    const [url, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('https://rxforge.example.com/oauth/token');
    expect(init.method).toBe('POST');
    expect((init.body as string)).toContain('grant_type=refresh_token');
    expect((init.body as string)).toContain('refresh_token=old-refresh-token');
  });

  it('throws when refresh token is invalid', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      text: async () => 'invalid_refresh_token',
    });

    await expect(
      refreshAccessToken('https://rxforge.example.com', 'client-123', 'expired-refresh')
    ).rejects.toThrow('Token refresh failed: invalid_refresh_token');
  });

  it('sends correct client_id in refresh request', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        access_token: 'tok',
        refresh_token: 'rtok',
        expires_in: 1800,
        token_type: 'Bearer',
      }),
    });

    await refreshAccessToken('https://rxforge.example.com', 'my-client-id', 'ref-tok');

    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect((init.body as string)).toContain('client_id=my-client-id');
  });
});

describe('getToken', () => {
  it('uses stored token if not expired', async () => {
    const stored = {
      accessToken: 'valid-access',
      refreshToken: 'valid-refresh',
      expiresAt: Date.now() + 120_000,
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('rxforge_token', JSON.stringify(stored));

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
    });

    expect(token).toBe('valid-access');
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it('refreshes token when it is within the expiry window', async () => {
    const stored = {
      accessToken: 'old-access',
      refreshToken: 'valid-refresh',
      expiresAt: Date.now() + 30_000, // inside the 60 s refresh threshold
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('rxforge_token', JSON.stringify(stored));

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        access_token: 'refreshed-access',
        refresh_token: 'new-refresh',
        expires_in: 3600,
        token_type: 'Bearer',
      }),
    });

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
      storagePrefix: 'rxforge',
    });

    expect(token).toBe('refreshed-access');
    expect(mockFetch).toHaveBeenCalledOnce();

    const saved = JSON.parse(localStorageMock.getItem('rxforge_token') as string);
    expect(saved.accessToken).toBe('refreshed-access');
    expect(saved.refreshToken).toBe('new-refresh');
    expect(saved.expiresAt).toBeGreaterThan(Date.now() + 3_000_000);
  });

  it('refreshes token when already expired', async () => {
    const stored = {
      accessToken: 'stale-access',
      refreshToken: 'good-refresh',
      expiresAt: Date.now() - 10_000, // already expired
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('rxforge_token', JSON.stringify(stored));

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        access_token: 'fresh-access',
        refresh_token: 'fresh-refresh',
        expires_in: 3600,
        token_type: 'Bearer',
      }),
    });

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
    });

    expect(token).toBe('fresh-access');
  });

  it('honors custom storagePrefix', async () => {
    const stored = {
      accessToken: 'prefixed-access',
      refreshToken: 'prefixed-refresh',
      expiresAt: Date.now() + 120_000,
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('myapp_token', JSON.stringify(stored));

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
      storagePrefix: 'myapp',
    });

    expect(token).toBe('prefixed-access');
  });

  it('coalesces concurrent calls into a single refresh', async () => {
    const stored = {
      accessToken: 'old-access',
      refreshToken: 'valid-refresh',
      expiresAt: Date.now() + 10_000, // triggers refresh
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('concurrent_token', JSON.stringify(stored));

    let resolveFetch: (v: unknown) => void = () => {};
    const fetchPromise = new Promise((resolve) => { resolveFetch = resolve; });
    mockFetch.mockReturnValueOnce(fetchPromise as any);

    const { getToken } = await import('./auth.js');
    const opts = {
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
      storagePrefix: 'concurrent',
    };

    const p1 = getToken(opts);
    const p2 = getToken(opts);
    const p3 = getToken(opts);

    resolveFetch({
      ok: true,
      json: async () => ({
        access_token: 'shared-access',
        refresh_token: 'shared-refresh',
        expires_in: 3600,
        token_type: 'Bearer',
      }),
    });

    const [t1, t2, t3] = await Promise.all([p1, p2, p3]);
    expect(t1).toBe('shared-access');
    expect(t2).toBe('shared-access');
    expect(t3).toBe('shared-access');
    // Despite three callers, only ONE refresh network call must have been made.
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it('falls back to full OAuth flow when refresh fails', async () => {
    const stored = {
      accessToken: 'dead-access',
      refreshToken: 'revoked-refresh',
      expiresAt: Date.now() - 1000,
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('fallback_token', JSON.stringify(stored));

    // First fetch: refresh fails.
    mockFetch.mockResolvedValueOnce({
      ok: false,
      text: async () => 'invalid_refresh_token',
    });
    // Second fetch: code->token exchange succeeds.
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        access_token: 'after-reauth',
        refresh_token: 'after-reauth-r',
        expires_in: 3600,
        token_type: 'Bearer',
      }),
    });

    // Stub window.open so that the popup appears to redirect to
    // redirect_uri?code=...&state=... immediately. startOAuthFlow's
    // polling loop picks up the code and resolves.
    const openMock = vi.fn((url: string) => {
      const params = new URL(url).searchParams;
      const state = params.get('state');
      const redirect = params.get('redirect_uri');
      return {
        closed: false,
        location: { href: `${redirect}?code=authz-code-123&state=${state}` },
        close() { this.closed = true; },
      };
    });
    (globalThis.window as unknown as { open: unknown }).open = openMock;

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      redirectUri: 'https://myapp.com/callback',
      storagePrefix: 'fallback',
    });

    expect(token).toBe('after-reauth');
    // Two fetches: the failed refresh + the code-exchange
    expect(mockFetch).toHaveBeenCalledTimes(2);
    expect(openMock).toHaveBeenCalledOnce();

    // Verify the code-exchange body included our simulated authorization code
    const [, exchangeInit] = mockFetch.mock.calls[1] as [string, RequestInit];
    expect((exchangeInit.body as string)).toContain('code=authz-code-123');
  });

  it('uses window.location.origin for default redirectUri when stored token is present', async () => {
    // Stored unexpired token means we never actually hit redirectUri code paths
    // but the function still computes it without throwing.
    const stored = {
      accessToken: 'ok-access',
      refreshToken: 'ok-refresh',
      expiresAt: Date.now() + 600_000,
      tokenType: 'Bearer',
    };
    localStorageMock.setItem('rxforge_token', JSON.stringify(stored));

    const { getToken } = await import('./auth.js');
    const token = await getToken({
      serverUrl: 'https://rxforge.example.com',
      clientId: 'client-123',
      // redirectUri intentionally omitted → default from window.location.origin
    });

    expect(token).toBe('ok-access');
  });
});
