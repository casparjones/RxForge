# RxForge Docker

This directory contains Docker configuration files for RxForge.

## Files

- `docker-compose.dev.yml` – Local development stack (includes CouchDB + PostgreSQL helper services)

## Production

The production `docker-compose.yml` is located in the project root. It runs only the `rxforge` backend container; CouchDB and PostgreSQL are expected to run externally (managed separately).

## Quick Start (Development)

```bash
cp .env.example .env
# Edit .env as needed
docker compose -f docker/docker-compose.dev.yml up -d
```

## Environment Variables

See `.env.example` in the project root for all required environment variables.
