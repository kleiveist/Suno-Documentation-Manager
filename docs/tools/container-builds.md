<!-- AUTO-GENERATED:backlink START -->
[← Back](tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Container builds and local production simulation — unavailable inherited reference

| Field | Value |
| --- | --- |
| Status | Archived |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Developers and operators |
| Related ATP | N/A — provider-neutral operations baseline |

> **Unavailable in this product:** This guide is retained from the upstream cloud-capable template. Suno Documentation Manager uses the `desktop-local` profile and has no backend, `deployment/` tree, Dockerfiles, Compose model, PostgreSQL capability, or container build target. The commands below are provenance examples and are not product procedures.

## Purpose

This guide provides the operational commands for building and validating the cloud-enabled container artifacts without introducing a vendor-specific deployment CLI.

## Scope

The guide covers Docker availability checks, image builds, Compose validation, static deployment, optional PostgreSQL, and explicit migrations. It does not cover a production cloud account or credential store.

## Procedure

Check the active profile and container tooling:

```sh
python tools/control.py container doctor
python tools/control.py container validate
```

The general `python tools/control.py doctor` includes these checks when `cloud` is enabled. A missing Docker executable reports a direct failure. Reduced non-cloud profiles report that container support is disabled instead of treating deployment files as missing.

Build both images or one component:

```sh
python tools/control.py build container
python tools/control.py build container --component backend
VITE_API_BASE_URL='https://api.example.invalid' \
  python tools/control.py build container --component frontend
```

The `.invalid` domain is an example and does not resolve. `VITE_API_BASE_URL` becomes visible in the browser bundle. Do not pass secrets as build arguments.

For static hosting, skip the frontend image:

```sh
VITE_API_BASE_URL='https://api.example.invalid' python tools/control.py build web
```

Upload the contents of `frontend/dist/` to the chosen static host. Configure that host to return `index.html` for application routes and to apply product-specific caching and security headers.

The Compose procedure and migration order are defined in [Deployment architecture](../def/deployment-architecture.md#local-production-simulation).

## Updating production dependency locks

The `*-production.txt` files are reviewed direct inputs. The matching `.lock` files pin the complete dependency graph and hashes used by Docker. Because conditional dependencies differ by interpreter version, regenerate all affected locks with `pip-tools` under Python 3.11, matching the backend container, from the repository root:

```sh
python3.11 -m piptools compile --generate-hashes \
  --output-file backend/requirements-production.lock \
  backend/requirements-production.txt
python3.11 -m piptools compile --generate-hashes \
  --output-file backend/requirements-database-production.lock \
  backend/requirements-database-production.txt
python3.11 -m piptools compile --generate-hashes \
  --output-file backend/requirements-postgres-production.lock \
  backend/requirements-postgres-production.txt
```

Review dependency and hash changes, validate them for Python 3.11, and rebuild the image. Do not edit generated lock entries by hand.

## Production configuration checklist

- Set `APP_ENV=production`.
- Set an explicit `APP_NAME` and narrow `BACKEND_CORS_ORIGINS`.
- Set the public `VITE_API_BASE_URL` before the frontend build.
- Inject `DATABASE_URL` only into backend and migration workloads when PostgreSQL is enabled.
- Terminate TLS at a trusted ingress or platform boundary.
- Run exactly one controlled Alembic migration job per rollout.
- Route traffic only to instances whose `/api/ready` probe succeeds.
- Replace local Compose credentials and image names before external use.
- Keep `.env` and secret values outside images, Git, build logs, and frontend assets.

## Verification

```sh
python tools/control.py container validate
python tools/control.py test --suite api
python tools/control.py test --suite tools
```

An environment with Docker additionally runs:

```sh
python tools/control.py build container
docker compose --file deployment/compose.yaml config --quiet
```

## Related documents

- [Deployment architecture](../def/deployment-architecture.md)
- [Runtime configuration](../def/configuration.md)
- [Database feature](../def/database-feature.md)
- [Release model](release-model.md)
