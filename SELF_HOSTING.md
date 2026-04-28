# Self-hosting RxForge

RxForge is designed to be deployed with `docker-compose`. The bundled
`docker-compose.yml` builds and runs the `rxforge` service only; CouchDB and
PostgreSQL are expected to run **externally** (on another host, managed
service, or a separate compose file). This keeps the data tier decoupled
from the app tier so that upgrades and backups can be managed independently.

## Prerequisites

- Docker and docker-compose (v2+).
- A reachable PostgreSQL 14+ instance.
- A reachable CouchDB 3.x instance with an admin user.
- A writable `./keys` directory that is bind-mounted into the container at
  `/app/keys`. **RxForge auto-generates an RSA-2048 keypair on first startup**
  if no keys are found there — no manual step required. The files
  `keys/private.pem` and `keys/public.pem` are created automatically and
  persist across restarts via the bind-mount.

  If you prefer to bring your own keypair (e.g. to share the same keys across
  multiple instances), place them there before starting:

  ```bash
  mkdir -p keys
  openssl genpkey -algorithm RSA -out keys/private.pem -pkeyopt rsa_keygen_bits:2048
  openssl rsa -in keys/private.pem -pubout -out keys/public.pem
  ```

  > **Important:** mount `./keys` as read-write (the default in
  > `docker-compose.yml`). A read-only mount prevents auto-generation and
  > will cause the server to fail on first start if the files are missing.

## 1. Configure the environment

Copy the example env file and edit it:

```bash
cp .env.example .env
```

Required variables (see `.env.example`):

| Variable | Example | Notes |
| --- | --- | --- |
| `DATABASE_URL` | `postgresql://rxforge:rxforge@db.internal:5432/rxforge` | PostgreSQL DSN used for metadata (apps, users, OAuth clients). |
| `COUCHDB_URL` | `http://couchdb.internal:5984` | Base URL of the external CouchDB. |
| `COUCHDB_USER` | `admin` | CouchDB admin user. |
| `COUCHDB_PASSWORD` | `s3cret` | CouchDB admin password. |
| `JWT_PRIVATE_KEY_PATH` | `/app/keys/private.pem` | Path **inside** the container. The compose file mounts `./keys` to `/app/keys` read-only. |
| `JWT_PUBLIC_KEY_PATH` | `/app/keys/public.pem` | Same as above. |
| `SERVER_PORT` | `8080` | Port inside the container; the compose file publishes `8080:8080`. |
| `FRONTEND_DIR` | `/app/dist` | Path to the built admin dashboard (static files). The compose file mounts `./dist` to `/app/dist` read-only. Produce the files with `npm run build`. |
| `RUST_LOG` | `info` | Log level. |

## 2. Build the frontend (optional but recommended)

The compose file mounts `./dist` into the container so the Rust server can
serve the admin dashboard as static files. Build it once before `docker
compose up`:

```bash
npm install
npm -w frontend run build
# output lands in ./frontend/build – symlink or copy into ./dist
ln -sfn frontend/build dist
```

If you do not need the admin dashboard, leave `./dist` empty; the API will
still work.

## 3. Prepare the external databases

Inside your PostgreSQL instance, create the database and user referenced by
`DATABASE_URL`:

```sql
CREATE USER rxforge WITH PASSWORD 'rxforge';
CREATE DATABASE rxforge OWNER rxforge;
```

Migrations live in `backend/migrations` and are applied by the server on
startup.

Inside CouchDB, RxForge will create one database per registered app on
demand – it only needs admin credentials, no manual setup.

## 4. Bring it up

### docker-compose

```bash
docker compose up -d --build
docker compose logs -f rxforge
```

### CapRover

Deploy via the CapRover dashboard or CLI. After creating the app, configure
the following before the first deploy:

**Persistent Directories** (App Config → Persistent Directories):

| Path in container | Notes |
| --- | --- |
| `/app/keys` | RSA keypair for JWT signing. Auto-generated on first start — must be persistent so keys survive redeploys. |

> `VOLUME` in a Dockerfile does **not** provision storage automatically in
> CapRover. You must add `/app/keys` as a Persistent Directory in the app
> settings. Without it, a new keypair is generated on every redeploy and all
> existing JWTs are immediately invalidated.

**Environment variables** — set these in App Config → Environmental Variables:

```
DATABASE_URL=postgresql://...
COUCHDB_URL=http://...
COUCHDB_USER=admin
COUCHDB_PASSWORD=...
JWT_PRIVATE_KEY_PATH=/app/keys/private.pem
JWT_PUBLIC_KEY_PATH=/app/keys/public.pem
```

The container exposes:

- `:8080/api/v1/…` – REST + replication API used by the plugin.
- `:8080/oauth/authorize` – OAuth authorization (browser redirect)
- `:8080/api/v1/oauth/token` – token exchange endpoint
- `:8080/` – admin dashboard (if `./dist` is populated).
- `:8080/health` – health probe used by the built-in `healthcheck`.

## 5. Verify

```bash
curl -fsS http://localhost:8080/health
```

Then open `http://localhost:8080/` in a browser, sign in as the bootstrap
admin, and register your first app + OAuth client. Use the resulting
`appId` and `clientId` when configuring the `rxdb-plugin-rxforge` plugin in
your client app.

## Backups and upgrades

Because the state lives in the external PostgreSQL and CouchDB, upgrading
RxForge is as simple as:

```bash
git pull
docker compose build rxforge
docker compose up -d rxforge
```

Back up PostgreSQL (`pg_dump`) and CouchDB (replicate to a backup instance
or use `couchdb-dump`) on your normal cadence – the `rxforge` container
holds no persistent state of its own, aside from the mounted `./keys`.

## Reverse proxy

For production, put RxForge behind a TLS-terminating reverse proxy
(Caddy, nginx, Traefik). The SSE endpoint (`/api/v1/sync/:app/stream`)
needs proxy buffering disabled – for nginx:

```nginx
location /api/v1/sync/ {
    proxy_pass http://rxforge:8080;
    proxy_http_version 1.1;
    proxy_set_header Connection '';
    proxy_buffering off;
    proxy_read_timeout 24h;
}
```
