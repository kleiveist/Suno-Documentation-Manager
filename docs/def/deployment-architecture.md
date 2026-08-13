<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Deployment architecture — unavailable inherited reference

| Field | Value |
| --- | --- |
| Status | Archived |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Developers, operators, and architects |
| Related ATP | N/A — template-level deployment baseline |

> **Unavailable in this product:** This page is retained solely as upstream template provenance. Suno Documentation Manager contains no `deployment/` or `backend/` directory, container images, Compose model, cloud runtime, FastAPI service, or PostgreSQL deployment unit. Product delivery is a local Tauri package as documented in the [release model](../tools/release-model.md); commands and paths below are not available product operations.

## Purpose

This document defines the provider-neutral cloud boundary, production artifacts, health contracts, and migration order. It explains which deployment units may be operated independently without selecting a cloud vendor.

## Scope

### Included

- static Vite artifacts and an optional non-root web server image;
- a non-root FastAPI image;
- local production simulation with Docker Compose;
- optional PostgreSQL, explicit Alembic migrations, and health probes; and
- profile ownership of deployment files.

### Excluded

- cloud accounts, Kubernetes, infrastructure provisioning, domains, TLS termination, managed databases, and production credentials.

## Deployment units

```mermaid
flowchart TD
    Template[Master template]
    Profile[Resolved project profile]
    Static[Vite dist artifact]
    API[FastAPI container]
    Browser[Static host or frontend container]
    Desktop[Tauri package]
    Database[(Optional PostgreSQL)]

    Template --> Profile
    Profile --> Static
    Profile --> API
    Profile --> Desktop
    Static --> Browser
    Browser -->|public HTTPS API| API
    Desktop -->|public HTTPS API when configured| API
    API -.->|only when postgres is enabled| Database
```

The Vite build, FastAPI service, native desktop package, and database are separate deployment units. The frontend contains no Python runtime. The desktop package does not embed or supervise FastAPI. A product may deploy only the units enabled by its profile.

## Static frontend artifact

The primary web artifact is independent of Docker:

```text
Vite source
    |
    v
frontend/dist/
    |
    v
static hosting or CDN
```

Build it from the project root:

```sh
python tools/control.py build web
```

The build also creates a ZIP under `.dist/web/`. `VITE_API_BASE_URL` is public build-time configuration. It must contain only a browser-visible URL. Credentials, database URLs, private keys, and access tokens must never use a `VITE_` name.

`deployment/docker/frontend.Dockerfile` is the optional container boundary. It builds with `npm ci`, copies only `dist/` into an unprivileged Nginx image, serves on port `8080`, and supports single-page application fallback. The runtime image receives no environment secrets.

## FastAPI image

`deployment/docker/backend.Dockerfile` uses a two-stage Python image. It installs fully pinned, hash-verified production lockfiles separately from source code, adds database and PostgreSQL packages only when their feature-owned lockfiles exist, and runs Uvicorn as UID/GID `10001`. Test dependencies are not installed from `backend/requirements.txt`.

Runtime configuration uses the existing environment contract. The image does not copy `.env` and does not embed credentials. The default command starts one Uvicorn process. Scale and process supervision belong to the selected runtime platform.

Build enabled images through the public CLI:

```sh
python tools/control.py container doctor
python tools/control.py container validate
python tools/control.py build container
python tools/control.py build container --component backend
python tools/control.py build container --component frontend
```

Container builds fail clearly when the active profile does not contain `cloud`. `web-only` and `desktop-local` therefore need neither Docker nor PostgreSQL.

## Local production simulation

The default Compose model starts frontend and backend without PostgreSQL:

```sh
docker compose --file deployment/compose.yaml up --build
```

PostgreSQL is behind the explicit `postgres` Compose profile. Use test-only local values and keep them out of Git:

```sh
export DATABASE_URL='postgresql+psycopg://app:local-only@postgres:5432/app'
export POSTGRES_PASSWORD='local-only'
docker compose --file deployment/compose.yaml --profile postgres build
docker compose --file deployment/compose.yaml --profile postgres up -d postgres
docker compose --file deployment/compose.yaml --profile postgres run --rm backend \
  python ../tools/control.py db upgrade
docker compose --file deployment/compose.yaml --profile postgres up -d backend frontend
```

The values above are local examples, not production credentials. A production platform injects its database URL and secrets through its secret manager or protected runtime environment.

## Health contracts

| Endpoint | Success | Dependency policy | Failure body |
| --- | --- | --- | --- |
| `GET /api/health` | HTTP 200 | Process only; never checks PostgreSQL | Stable process status |
| `GET /api/ready` | HTTP 200 | Checks `SELECT 1` when the generated profile enables `database` | HTTP 503 with `{"status":"unavailable"}` |

Readiness does not include database hosts, driver errors, credentials, stack traces, or other infrastructure details. The backend container health check uses liveness so a temporary database outage does not cause an automatic process restart loop. A platform may route traffic using readiness separately.

## Migration order

Application replicas never run Alembic automatically. One controlled deployment step performs migrations before new application instances receive traffic:

```text
deploy image
    |
    v
run one migration job: python tools/control.py db upgrade
    |
    v
start or roll application replicas
    |
    v
enable readiness traffic
```

The migration job uses the same image and server-side `DATABASE_URL` as the application. Operators review downgrade compatibility and backups before applying destructive schema changes.

## Profile behavior

| Profile | Static web build | Backend image | Desktop build | PostgreSQL profile |
| --- | ---: | ---: | ---: | ---: |
| `web-only` | Yes | No | No | Not compatible |
| `web-cloud` | Yes | Yes | No | Optional |
| `desktop-local` | Yes | No | Yes | Not compatible |
| `desktop-cloud` | Yes | Yes | Yes | Optional |
| `full-platform` | Yes | Yes | Yes | Optional |

Selecting `--with postgres` adds SQLAlchemy, Alembic, Psycopg, database readiness, and PostgreSQL Compose usage. It does not make PostgreSQL a requirement for profiles that did not select it.

## Verification

```sh
python tools/control.py version check
python tools/control.py container validate
python tools/control.py build web
python tools/control.py build container
curl --fail http://127.0.0.1:8000/api/health
curl --fail http://127.0.0.1:8000/api/ready
```

Docker is required only for the container commands.

## Risks and limitations

- Image tags are baseline inputs, not an organization-specific software bill of materials or vulnerability policy.
- TLS, ingress, backups, secret rotation, log shipping, and metrics belong to the selected production platform.
- The local Compose password defaults are intentionally non-production values and must never be reused externally.

## Related documents

- [Framework architecture](architecture.md)
- [Runtime configuration](configuration.md)
- [Database feature](database-feature.md)
- [Container operations](../tools/container-builds.md)
- [Release model](../tools/release-model.md)
