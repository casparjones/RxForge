import type { RxCollection, RxConflictHandler } from 'rxdb';

/**
 * Configuration options for the RxForge replication plugin.
 */
export interface RxForgeReplicationOptions<RxDocType> {
  /** The RxDB collection to replicate */
  collection: RxCollection<RxDocType>;

  /** Base URL of the RxForge server (e.g. https://your-rxforge.example.com) */
  serverUrl: string;

  /** The app ID registered in RxForge */
  appId: string;

  /** The OAuth client_id for this app */
  clientId: string;

  /** Optional OAuth redirect URI. Defaults to window.location.origin + '/oauth/callback' */
  redirectUri?: string;

  /** Number of documents to fetch per pull batch. Default: 100 */
  batchSize?: number;

  /** Optional custom conflict handler. Defaults to last-write-wins by updatedAt */
  conflictHandler?: RxConflictHandler<RxDocType>;

  /** Storage key prefix for tokens in localStorage. Default: 'rxforge' */
  storagePrefix?: string;
}

/**
 * Stored token data in localStorage.
 */
export interface StoredToken {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp in ms
  tokenType: string;
}

/**
 * OAuth token response from the server.
 */
export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
}

/**
 * Pull result shape expected from RxForge server.
 */
export interface PullResult<RxDocType> {
  documents: RxDocType[];
  checkpoint: unknown;
}
