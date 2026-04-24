import type { RxForgeReplicationOptions, StoredToken, TokenResponse } from './types.js';

const REFRESH_BEFORE_EXPIRY_MS = 60_000; // 60 seconds

function getStorageKey(prefix: string): string {
  return `${prefix}_token`;
}

function readToken(prefix: string): StoredToken | null {
  try {
    const raw = localStorage.getItem(getStorageKey(prefix));
    if (!raw) return null;
    return JSON.parse(raw) as StoredToken;
  } catch {
    return null;
  }
}

function writeToken(prefix: string, token: StoredToken): void {
  localStorage.setItem(getStorageKey(prefix), JSON.stringify(token));
}

function clearToken(prefix: string): void {
  localStorage.removeItem(getStorageKey(prefix));
}

function isExpired(token: StoredToken): boolean {
  return Date.now() >= token.expiresAt - REFRESH_BEFORE_EXPIRY_MS;
}

/**
 * Exchange an OAuth authorization code for access + refresh tokens.
 */
export async function exchangeCodeForToken(
  serverUrl: string,
  clientId: string,
  code: string,
  redirectUri: string
): Promise<TokenResponse> {
  const res = await fetch(`${serverUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'authorization_code',
      client_id: clientId,
      code,
      redirect_uri: redirectUri,
    }).toString(),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Token exchange failed: ${text}`);
  }
  return res.json() as Promise<TokenResponse>;
}

/**
 * Refresh an access token using the refresh token.
 */
export async function refreshAccessToken(
  serverUrl: string,
  clientId: string,
  refreshToken: string
): Promise<TokenResponse> {
  const res = await fetch(`${serverUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'refresh_token',
      client_id: clientId,
      refresh_token: refreshToken,
    }).toString(),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Token refresh failed: ${text}`);
  }
  return res.json() as Promise<TokenResponse>;
}

/**
 * Start the OAuth Authorization Code flow by opening a popup window.
 * Resolves with the authorization code once the popup redirects back.
 */
export function startOAuthFlow(
  serverUrl: string,
  clientId: string,
  redirectUri: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const state = Math.random().toString(36).slice(2);
    const params = new URLSearchParams({
      client_id: clientId,
      redirect_uri: redirectUri,
      response_type: 'code',
      state,
    });

    const url = `${serverUrl}/oauth/authorize?${params.toString()}`;
    const popup = window.open(url, 'rxforge_oauth', 'width=520,height=600,left=200,top=100');

    if (!popup) {
      reject(new Error('Failed to open OAuth popup. Check popup blocker settings.'));
      return;
    }

    function onMessage(event: MessageEvent) {
      if (event.origin !== new URL(serverUrl).origin) return;
      const { code, state: retState, error } = event.data as { code?: string; state?: string; error?: string };
      if (retState !== state) return;
      cleanup();
      if (error) {
        reject(new Error(`OAuth error: ${error}`));
      } else if (code) {
        resolve(code);
      }
    }

    // Fallback: poll popup URL for redirect
    const pollInterval = setInterval(() => {
      try {
        if (!popup || popup.closed) {
          cleanup();
          reject(new Error('OAuth popup was closed before completing authentication.'));
          return;
        }
        const popupUrl = popup.location.href;
        const popupParams = new URL(popupUrl);
        const code = popupParams.searchParams.get('code');
        const retState = popupParams.searchParams.get('state');
        if (code && retState === state) {
          popup.close();
          cleanup();
          resolve(code);
        }
      } catch {
        // Cross-origin – ignore until redirect completes
      }
    }, 500);

    function cleanup() {
      clearInterval(pollInterval);
      window.removeEventListener('message', onMessage);
    }

    window.addEventListener('message', onMessage);
  });
}

// In-flight token request coalescing. Ensures concurrent callers share the
// same refresh / OAuth attempt rather than issuing parallel network calls.
const inFlight = new Map<string, Promise<string>>();

async function resolveToken<RxDocType>(
  options: Pick<RxForgeReplicationOptions<RxDocType>, 'serverUrl' | 'clientId' | 'redirectUri' | 'storagePrefix'>
): Promise<string> {
  const prefix = options.storagePrefix ?? 'rxforge';
  const redirectUri = options.redirectUri ?? `${window.location.origin}/oauth/callback`;

  let stored = readToken(prefix);

  if (stored && !isExpired(stored)) {
    return stored.accessToken;
  }

  if (stored?.refreshToken) {
    try {
      const refreshed = await refreshAccessToken(options.serverUrl, options.clientId, stored.refreshToken);
      stored = {
        accessToken: refreshed.access_token,
        refreshToken: refreshed.refresh_token,
        expiresAt: Date.now() + refreshed.expires_in * 1000,
        tokenType: refreshed.token_type,
      };
      writeToken(prefix, stored);
      return stored.accessToken;
    } catch {
      // Refresh failed → full re-auth
      clearToken(prefix);
    }
  }

  // Full OAuth flow
  const code = await startOAuthFlow(options.serverUrl, options.clientId, redirectUri);
  const tokenResponse = await exchangeCodeForToken(options.serverUrl, options.clientId, code, redirectUri);
  const newToken: StoredToken = {
    accessToken: tokenResponse.access_token,
    refreshToken: tokenResponse.refresh_token,
    expiresAt: Date.now() + tokenResponse.expires_in * 1000,
    tokenType: tokenResponse.token_type,
  };
  writeToken(prefix, newToken);
  return newToken.accessToken;
}

/**
 * Get a valid access token for the given options.
 * Will refresh or re-authenticate as necessary.
 * Concurrent calls for the same storage prefix are coalesced into one request.
 */
export function getToken<RxDocType>(
  options: Pick<RxForgeReplicationOptions<RxDocType>, 'serverUrl' | 'clientId' | 'redirectUri' | 'storagePrefix'>
): Promise<string> {
  const prefix = options.storagePrefix ?? 'rxforge';
  const existing = inFlight.get(prefix);
  if (existing) return existing;

  const promise = resolveToken(options).finally(() => {
    inFlight.delete(prefix);
  });
  inFlight.set(prefix, promise);
  return promise;
}
