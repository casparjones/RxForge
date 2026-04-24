# rxdb-plugin-rxforge

Offline-first replication for [RxDB](https://rxdb.info) against a self-hosted
[RxForge](https://github.com/) sync server. This package provides a single
`replicateRxForge()` helper that wires up pull, push, SSE-based pullStream,
OAuth Authorization Code authentication, automatic token refresh, and a
last-write-wins conflict handler.

## Install

```bash
npm install rxdb-plugin-rxforge
```

`rxdb >= 15` is a peer dependency:

```bash
npm install rxdb
```

## Getting started

```ts
import { createRxDatabase } from 'rxdb';
import { getRxStorageDexie } from 'rxdb/plugins/storage-dexie';
import { replicateRxForge } from 'rxdb-plugin-rxforge';

const db = await createRxDatabase({
  name: 'mydb',
  storage: getRxStorageDexie(),
});

await db.addCollections({
  todos: {
    schema: todoSchema,
  },
});

const replicationState = replicateRxForge({
  collection: db.todos,
  serverUrl: 'https://rxforge.example.com',
  appId: 'my-app-id',
  clientId: 'my-oauth-client-id',

  // Optional
  redirectUri: 'https://myapp.example.com/oauth/callback',
  batchSize: 100,
  storagePrefix: 'rxforge',
  conflictHandler: async ({ newDocumentState, realMasterState }) => {
    // Custom merge strategy (optional – default is LWW by updatedAt).
    return {
      documentData:
        newDocumentState.updatedAt >= realMasterState.updatedAt
          ? newDocumentState
          : realMasterState,
    };
  },
});

await replicationState.awaitInitialReplication();
```

On first use, the plugin opens a popup window to
`${serverUrl}/oauth/authorize`. Once the user approves, the popup redirects to
your `redirectUri` with `?code=…&state=…`, and the plugin exchanges the code
for an access + refresh token. Tokens are stored in `localStorage` and
refreshed automatically ~60 seconds before expiry. If the refresh token is no
longer valid, the OAuth popup is re-opened.

## OAuth setup in the RxForge dashboard

Before your app can replicate, it needs to be registered in RxForge:

1. Log in to the RxForge admin dashboard.
2. Create a new **App** and note its app ID – this becomes `appId`.
3. Register an **OAuth client** for the app and note its client ID – this
   becomes `clientId`. RxForge uses the public-client PKCE-less Authorization
   Code flow, so no client secret is required in the browser.
4. Add every origin+path that your app will use as an allowed
   **Redirect URI** (e.g. `https://myapp.example.com/oauth/callback` and
   `http://localhost:5173/oauth/callback` for development).
5. Grant the client access to one or more **collections** on the app so that
   pull/push are authorised.

Your `redirectUri` page needs to do one of two things after the OAuth provider
redirects back to it:

- Do nothing: the plugin polls `popup.location.href` and detects the
  redirect itself. A blank HTML page is enough.
- Or `window.opener.postMessage({ code, state }, '<serverUrl origin>')` and
  close the popup – the plugin listens for this message as well.

## Configuration options

All options on `RxForgeReplicationOptions<RxDocType>`:

| Option | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `collection` | `RxCollection<RxDocType>` | yes | – | The RxDB collection to replicate. |
| `serverUrl` | `string` | yes | – | Base URL of the RxForge server, e.g. `https://rxforge.example.com`. No trailing slash. |
| `appId` | `string` | yes | – | The app ID registered in the RxForge dashboard. Used in the sync URL and as part of the replication identifier. |
| `clientId` | `string` | yes | – | The OAuth client_id registered for this app. |
| `redirectUri` | `string` | no | `${window.location.origin}/oauth/callback` | OAuth redirect URI. Must be listed as an allowed redirect in RxForge. |
| `batchSize` | `number` | no | `100` | Number of documents fetched per pull page and sent per push batch. |
| `conflictHandler` | `RxConflictHandler<RxDocType>` | no | last-write-wins by `updatedAt` (tie → new document) | Custom conflict resolver. Installed on the collection. |
| `storagePrefix` | `string` | no | `'rxforge'` | Prefix for the `localStorage` key (`${prefix}_token`). Use distinct values when replicating against multiple RxForge servers from the same origin. |

## How it works

- **pullHandler** — `GET ${serverUrl}/api/v1/sync/${appId}/pull?checkpoint=<json>&limit=<n>` with `Authorization: Bearer <token>`. Response: `{ documents, checkpoint }`.
- **pushHandler** — `POST ${serverUrl}/api/v1/sync/${appId}/push` with the array of `RxReplicationWriteToMasterRow` rows as JSON. Response: array of conflicting master documents (empty on success).
- **pullStream$** — Connects to `GET ${serverUrl}/api/v1/sync/${appId}/stream?token=<token>` via `EventSource`. Emits `change` events carrying `{ documents, checkpoint }`. The stream reconnects automatically after 5 s on error.
- **replicationIdentifier** — `rxforge-${appId}-${collection.name}`.

## License

MIT
