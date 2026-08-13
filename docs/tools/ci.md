<!-- AUTO-GENERATED:backlink START -->
[← Back](tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Continuous integration — unavailable inherited reference

| Field | Value |
| --- | --- |
| Status | Archived |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Contributors and repository maintainers |
| Related ATP | N/A - automated evidence supports, but does not replace, feature ATPs |

> **Unavailable in this product:** This page describes the upstream master template's GitHub Actions design. The generated Suno Documentation Manager source tree contains no `.github/workflows/` directory, so none of the workflow names, matrices, remote checks, artifacts, or branch-protection recommendations below is current product CI evidence. Use the local verification commands and product ATP records until product-specific CI is added and executed.

## Purpose

This document defines the automated quality gates for the master template. Continuous integration detects regressions in shared tooling, feature modules, profile generation, PostgreSQL integration, container builds, production web builds, and native desktop packaging before a change reaches `main`.

## Scope

Core, profile, PostgreSQL, and desktop workflows run for pull requests and pushes to `main`. They validate the repository and temporary generated projects. The separate release-validation workflow runs only for a matching version tag or manual dispatch. Deployment, publishing, signing, release creation, and production database operations remain outside the automated baseline.

## Local and CI equivalence

GitHub Actions orchestrates the same public interface used by developers:

```text
GitHub Actions
      |
      v
python tools/control.py
      |
      +-- install
      +-- doctor / config doctor / tauri doctor
      +-- test --suite <name>
      +-- db upgrade
      +-- build web
      +-- container validate / build container
      +-- version check / release check
```

Project behavior remains in `tools/control.py` and its modules. Workflow YAML is responsible only for runtime setup, dependency caches, temporary service containers, command ordering, and job boundaries.

The local baseline is:

```sh
python tools/control.py install
python tools/control.py doctor
python tools/control.py test --suite all
python tools/control.py build web
```

`test --suite all` reads `project-profile.toml`. Disabled features report `SKIP` and return success. Missing files or failed checks for enabled features report `FAIL` and return a non-zero exit code.

Tests that exercise generator capabilities outside a derived project's enabled feature set are skipped. The same tooling suite still runs in every generated project; only master-only source-completeness checks are excluded when the corresponding source modules were intentionally not scaffolded.

## Test levels

| Level | Coverage | Public command |
| --- | --- | --- |
| Core tests | Tooling, profiles, configuration, schema, FastAPI, SQLAlchemy, frontend, Tauri, and Rust | `python tools/control.py test --suite <name>` |
| External service tests | PostgreSQL connectivity and Alembic migration | `python tools/control.py test --suite postgres`, `python tools/control.py db upgrade` |
| Generated project tests | Real scaffolds for every supported profile | `python tools/control.py init`, followed by generated-project commands |
| Build verification | Vite, provider-neutral images, and native Tauri packages | `python tools/control.py build web`, `build container`, `build desktop` |

Playwright E2E remains an optional suite. It reports `SKIP` until a real E2E configuration and tests exist.

## Workflows

### Core CI

`.github/workflows/ci.yml` separates failures into four jobs:

| Check name | Responsibility |
| --- | --- |
| `Core / Tooling, Profiles & Configuration` | CLI, generator, profile, configuration, and workflow regression tests |
| `Core / Backend, Database & Schema` | JSON Schema, FastAPI, and SQLAlchemy unit tests |
| `Core / Frontend & Web Build` | Vitest and a production Vite build |
| `Core / Container Build` | Version and Compose validation plus backend and frontend image builds |

The jobs are independent and run in parallel. Native packages belong to Desktop CI.

### Profile Matrix

`.github/workflows/profiles.yml` generates `web-only`, `web-cloud`, `desktop-local`, `desktop-cloud`, and `full-platform` projects. Each matrix entry runs an initial structure doctor, dependency installation, a prepared-environment doctor, the profile-aware complete suite, and a production web build. Desktop entries also install Linux Tauri prerequisites, run Tauri doctor, and execute Cargo checks. Cloud entries validate their generated Compose model.

On Debian-family Linux runners, `tauri install` refreshes apt metadata before a non-interactive install. It uses the Tauri v2 WebKitGTK 4.1 dependency set and selects the release-appropriate FUSE 2 package name (`libfuse2` on Ubuntu 22.04 and `libfuse2t64` on Ubuntu 24.04 and newer). Missing required compile-time libraries make `tauri doctor` fail; AppImage-only helpers remain warnings during check-only CI.

The matrix uses `fail-fast: false`. One broken preset therefore does not hide the status of the other presets.

### PostgreSQL Integration

`.github/workflows/postgres.yml` starts PostgreSQL 16 as an isolated service container with test-only credentials and a `pg_isready` health check. No external database is contacted.

The integration job performs this sequence:

```text
fresh temporary database
        |
        v
config doctor
        |
        v
Alembic upgrade head
        |
        v
PostgreSQL integration test
```

The same workflow generates `web-cloud`, `desktop-cloud`, and `full-platform` with `--with postgres`. Each generated project runs doctor, migration, the complete profile-aware suite, the web build, and container-model validation. Desktop entries also run Tauri checks. `DATABASE_URL` and `DATABASE_URL_TEST` point only to the job-local service.

Application startup never runs migrations. CI invokes `db upgrade` explicitly.

### Desktop CI

`.github/workflows/desktop.yml` uses a native matrix over Ubuntu, macOS, and Windows. Every runner installs its Tauri prerequisites, executes locked Rust checks, builds a technically available native package, and uploads a short-lived artifact named `desktop-<target>-unsigned`. Linux verifies a Debian package. No job reads signing or notarization secrets.

The artifacts establish build compatibility. They are not production releases and are not published outside the workflow artifact store.

### Release Validation

`.github/workflows/release.yml` accepts `workflow_dispatch` and tags matching `v*.*.*`. It runs the full validation suite, builds the web candidate, executes `release check`, and calls the same unsigned desktop workflow. It has read-only repository permission and no publication or deployment job. Product repositories add protected signing and publication jobs only after this gate.

## Runtime versions

| Runtime | CI baseline |
| --- | --- |
| Python | 3.11 |
| Node.js | 20 |
| Rust | Stable toolchain |
| PostgreSQL | 16 service container for integration tests |

These values test the minimum versions documented by the template for Python and Node.js and the supported stable Rust channel.

The official setup and cache actions use their Node.js 24-based stable majors. This internal action runtime is independent of the project-level Node.js 20 baseline above.

## Dependency installation and caching

`control.py install` uses `npm ci` whenever `frontend/package-lock.json` exists and falls back to `npm install` only for projects without a lockfile. Cargo commands use `--locked` with `src-tauri/Cargo.lock`. Python keeps the existing requirements-file and virtual-environment strategy.

GitHub-hosted caches cover:

- pip downloads keyed by the relevant requirements files;
- npm downloads keyed by `frontend/package-lock.json`;
- Cargo registry, Git sources, and target output keyed by `src-tauri/Cargo.lock`.

Caches improve execution time but are never required for correctness.

## Permissions and secrets

Every workflow declares only `contents: read`. Pull requests from forks do not require repository secrets. PostgreSQL uses the temporary user, password, and database declared in workflow YAML; those values exist only inside an isolated CI run and are not production credentials.

The workflows do not use cloud credentials, signing material, deployment tokens, package publication, public release creation, or write permissions. Commands must not print database URLs or other secret values. Normal CI jobs run `git diff --exit-code` to detect unintended changes to versioned files.

## Failure handling

Jobs use explicit timeouts. Pull-request concurrency cancels an older run after a newer commit arrives, while `main` runs are not cancelled. Independent jobs remain parallel so GitHub reports the failing subsystem directly.

`PASS`, `SKIP`, and `FAIL` have distinct meanings:

- `PASS`: the selected check ran and completed successfully.
- `SKIP`: the active profile disables the feature or an optional suite is not configured.
- `FAIL`: an enabled feature is incomplete, configuration is invalid, or the executed check failed.

A configured but unreachable `DATABASE_URL_TEST` is a failure, not a skip.

## Branch protection recommendation

Configure `main` branch protection after the workflows have completed stable repository runs. Require the actual checks shown by GitHub for:

- all four `Core / ...` jobs;
- all five `Profiles / ...` matrix jobs;
- `PostgreSQL / Integration & Migration`;
- all three `Profiles / ... + postgres` jobs; and
- all three `Desktop / ... / Unsigned Verification` jobs.

Require pull requests and current branch status before merge. Do not grant test workflows deployment or bypass privileges.

## Acceptance evidence

Automated tests and CI output provide repeatable technical evidence. They do not replace feature acceptance:

```text
Automated tests + CI evidence + acceptance testing = feature acceptance
```

Reference relevant CI runs from an ATP when they support an acceptance decision.

## Verification

Run the local equivalents from the repository root:

```sh
python tools/control.py test --suite tools
python tools/control.py test --suite schema
python tools/control.py test --suite api
python tools/control.py test --suite database
python tools/control.py test --suite frontend
python tools/control.py test --suite tauri
python tools/control.py build web
python tools/control.py container validate
python tools/control.py version check
```

PostgreSQL verification additionally requires a disposable test database through `DATABASE_URL` and `DATABASE_URL_TEST`.

## Related documents

- [Tooling Guide](tooling.md)
- [Framework Architecture](../def/architecture.md)
- [Project Profiles](../def/project-profiles.md)
- [Database Feature](../def/database-feature.md)
- [Runtime Configuration](../def/configuration.md)
- [Deployment Architecture](../def/deployment-architecture.md)
- [Release Model](release-model.md)
- [ATP Workflow](../atp/README.md)
