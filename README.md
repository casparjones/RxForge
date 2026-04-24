# RxForge

Self-hosted sync backend for RxDB apps. Provides OAuth 2.0, per-user CouchDB provisioning, and a TypeScript plugin for seamless RxDB replication.

## Structure

```
rxforge/
├── backend/     # Rust/axum HTTP service
├── frontend/    # SvelteKit admin dashboard (static)
├── plugin/      # npm package rxdb-plugin-rxforge
└── docker/      # Docker Compose configs
```

## Quick Start

See [SELF_HOSTING.md](./SELF_HOSTING.md) for Docker deployment instructions.

## Plugin Usage

```ts
import { replicateRxForge } from 'rxdb-plugin-rxforge';
```

See [plugin/README.md](./plugin/README.md) for full documentation.
