<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Upstream template final acceptance — historical reference

> **Inherited and unavailable reference:** This page was copied from the upstream master template and records that repository's historical acceptance only. Its commits, hashes, workflow links, environments, matrices, and decisions are not available Suno Documentation Manager evidence and must not be included in a product acceptance decision. Product acceptance is tracked in the [product ATP index](../atp/atp.md) and [product acceptance report](acceptance-report.md).

| Field | Value |
| --- | --- |
| Status | Archived |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Maintainers, reviewers, and acceptance owners |
| Related ATP | N/A — repository-wide final acceptance |

## Purpose

This protocol records the final technical acceptance of the reusable web, cloud, and desktop template. It distinguishes repository verification from checks that require external GitHub settings, native runners, signing credentials, or a usable container daemon.

## Acceptance status

**PASS WITH OPEN EXTERNAL ITEMS**

The repository implementation is accepted for the case-study phase. No repository-side P0 or P1 finding remains open. The tested functional commit is:

```text
1cdfb6e84367200766593194e2cd1e4249259823
```

The acceptance commit is not yet present on the remote repository. Current GitHub Actions runs therefore cover the preceding commit `7cb4de472f8cd162611ca6bfac1f70a578060567`, not the accepted changes.

## Scope

### Included

- master repository tooling, configuration, tests, builds, migrations, release checks, and hygiene;
- all five generated profiles and all three compatible PostgreSQL combinations;
- PostgreSQL incompatibility checks and deterministic generation;
- frontend, FastAPI, SQLAlchemy, Alembic, Psycopg, Tauri, Rust, Compose, and production dependency locks;
- GitHub workflow definitions and the latest available public workflow runs; and
- documentation, secret boundaries, placeholders, and release metadata.

### Excluded

- publishing, deployment, production signing, notarization, and updater activation;
- changes to GitHub branch protection or required status checks; and
- product-specific authentication, business models, cloud providers, and credentials.

## Environment

The browser and server profile matrix used Python 3.11.15 and Node.js 20.20.2. Native Linux desktop verification used Python 3.14.6, Node.js 26.4.0, Rust 1.97.1, Tauri CLI 2.11.4, WebKitGTK 4.1, and the required Linux packaging tools. Production dependency locks were resolved and installed under Python 3.11. PostgreSQL 16.13 ran as a disposable local test service on a non-default port. Docker Compose v5.4.0 validated the deployment model.

No production database, credential, release, deployment, or repository setting was changed.

## Profile matrix

| Profile | Generate | Install | Doctor | Tests | Web build | Tauri / package |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `web-only` | PASS | PASS | PASS | PASS | PASS | N/A |
| `web-cloud` | PASS | PASS | PASS WITH NOTE | PASS | PASS | N/A |
| `desktop-local` | PASS | PASS | PASS | PASS | PASS | PASS / Linux DEB |
| `desktop-cloud` | PASS | PASS | PASS WITH NOTE | PASS | PASS | PASS / Linux DEB |
| `full-platform` | PASS | PASS | PASS WITH NOTE | PASS | PASS | PASS / Linux DEB |

The cloud-profile Doctor note is limited to an optional local Docker/Compose tool warning when that tool is absent. With the temporary verified Compose plugin enabled, Doctor and `container validate` both pass. Feature-disabled suites report `SKIP`; enabled suites report `PASS` and missing enabled dependencies remain failures.

Structure inspection confirmed:

- `web-only` contains no backend, deployment, Tauri, or PostgreSQL dependency;
- `web-cloud` contains frontend, backend, and provider-neutral deployment sources without PostgreSQL;
- `desktop-local` contains frontend and Tauri sources without backend or cloud sources;
- `desktop-cloud` contains frontend, backend, Tauri, and deployment sources without PostgreSQL; and
- `full-platform` contains the complete baseline without enabling PostgreSQL by default.

## PostgreSQL matrix

| Profile | PostgreSQL | Migration | Tests | Container model | Desktop package |
| --- | ---: | ---: | ---: | ---: | ---: |
| `web-cloud + postgres` | PASS | PASS | PASS | PASS | N/A |
| `desktop-cloud + postgres` | PASS | PASS | PASS | PASS | PASS / Linux DEB |
| `full-platform + postgres` | PASS | PASS | PASS | PASS | PASS / Linux DEB |

Each combination passed configuration validation, dependency installation, a real `SELECT 1` connection probe, `db upgrade`, `db current`, SQLAlchemy tests, PostgreSQL integration tests, API tests, frontend tests, and the web build. Desktop combinations also passed Tauri/Rust tests and native Linux packaging.

`web-only + postgres` and `desktop-local + postgres` both return exit code `1`, explain that PostgreSQL requires the backend feature, and leave no partial target directory.

The template contains no business model and therefore no initial revision. Alembic still validates and executes `current` and `upgrade head`. Application startup does not invoke Alembic, database engines are lazy, and migrations share the declarative SQLAlchemy `Base`.

## Commands and results

The public command surface was exercised through `tools/control.py`. All command help pages returned exit code `0`. An unknown command returned exit code `2` with a concise parser error and no traceback.

Master verification included:

```sh
python tools/control.py install --skip-playwright
python tools/control.py doctor
python tools/control.py config show
python tools/control.py config doctor
python tools/control.py db doctor --connect
python tools/control.py db upgrade
python tools/control.py db current
python tools/control.py test --suite all
python tools/control.py build web
python tools/control.py container validate
python tools/control.py tauri doctor
python tools/control.py test --suite tauri
python tools/control.py version check
python tools/control.py release check
```

The clean-tree `release check` passed at the tested commit. Its only warning states that the verification build is unsigned. The master repository intentionally retains its canonical scaffold identity; generated projects still fail the release gate until product identity is customized.

Generated profile verification used the following sequence, with only feature-relevant commands enabled:

```sh
python tools/control.py doctor
python tools/control.py install --skip-playwright
python tools/control.py doctor
python tools/control.py test --suite all
python tools/control.py build web
python tools/control.py tauri doctor
python tools/control.py test --suite tauri
python tools/control.py build desktop --target linux --bundles deb
```

PostgreSQL profiles additionally ran:

```sh
python tools/control.py config doctor
python tools/control.py db doctor --connect
python tools/control.py db upgrade
python tools/control.py db current
```

The current tooling suite passed under a fresh Python 3.11 environment:

```text
189 passed
```

All three hash-locked production dependency sets installed sequentially with `pip --require-hashes` under Python 3.11, and imports of FastAPI, SQLAlchemy, Alembic, Psycopg, and Greenlet succeeded. The Compose model passed `docker compose config` for the master and every cloud profile.

The actual backend and frontend `docker build` operations are **NOT VERIFIED** for the accepted commit. The available host account cannot access `/var/run/docker.sock`. The failed command reported only Docker daemon permission denial after the dependency-lock defect had been corrected. Local Compose `up` is likewise **NOT VERIFIED**, and no container was started or left running.

## Configuration and security

Tests verify the documented priority order: CLI override, process environment, local `.env`, then contract default. Development, test, and production validation reject invalid environments, ports, hosts, CORS origins, public secrets, and missing PostgreSQL configuration. Profiles without PostgreSQL have no database requirement.

`config show` redacts the database password. Server-only values are removed from frontend and Tauri process environments. `DATABASE_URL` is absent from frontend environment output and non-PostgreSQL generated `.env.example` files. PostgreSQL examples contain only the explicit local `change-me` placeholder. Tests also confirm that encoded and decoded passwords are removed from diagnostics.

The frontend-to-backend contract covers `/api/health`, `/api/ready`, API base URL derivation, browser and Tauri CORS origins, stable unavailable responses, and database-independent liveness. Shared schemas remain framework-neutral.

## Reproducibility and hygiene

Two independent `desktop-cloud + postgres` projects generated from identical inputs were byte-for-byte identical according to recursive comparison. `init --dry-run` wrote no target. Existing non-empty targets are rejected. Incompatible optional features are validated before target creation.

Tracked-file and ignore checks found no committed `.env`, virtual environment, `node_modules`, `.generated`, `.dist`, `.report`, test cache, Rust target, or package artifact. `.dockerignore` excludes repository metadata, local configuration, tests, dependency directories, and build caches from image contexts.

The placeholder scan found only intentional canonical template identity, documented examples, replacement logic, and test fixtures. Product generation can replace names, slugs, identifiers, package metadata, Compose names, Tauri metadata, icons, binaries, and artifact names.

## GitHub Actions status

The latest public runs were inspected through the GitHub API. They all target the pre-acceptance commit `7cb4de472f8cd162611ca6bfac1f70a578060567`.

| Workflow | Run | Remote result | Acceptance finding |
| --- | --- | --- | --- |
| Core CI | [31157800383](https://github.com/kleiveist/Template-Projekte/actions/runs/31157800383) | FAIL | Backend and frontend passed. The container job exposed the Python 3.13/3.11 lock mismatch. Current Python 3.11 locks install successfully. The tooling failure did not reproduce in a clean Python 3.11 archive or the accepted tree. |
| Profile Matrix | [31157800470](https://github.com/kleiveist/Template-Projekte/actions/runs/31157800470) | FAIL | Web profiles passed. Current generated desktop profiles pass locally with native prerequisites. |
| PostgreSQL Integration | [31157800342](https://github.com/kleiveist/Template-Projekte/actions/runs/31157800342) | FAIL | The master integration job passed. Generated jobs exposed the inherited `DATABASE_URL` test defect; all three current combinations now pass against disposable PostgreSQL. |
| Desktop CI | [31157800430](https://github.com/kleiveist/Template-Projekte/actions/runs/31157800430) | FAIL | Linux and macOS passed. Windows failed while launching frontend installation; Windows `.cmd` launchers now use `cmd.exe`, with regression coverage. |
| Release Validation | N/A | NOT VERIFIED | No public tag or manual run exists for the accepted commit. The non-publishing gate passes locally. |

Workflow contract tests verify YAML loading, events, permissions, timeouts, supported runtime setup, caching, public `control.py` entry points, unsigned artifacts, and repository-diff guards. A remote rerun of the accepted commit is **NOT VERIFIED** because the environment has no authenticated GitHub CLI or token and the commit was not pushed during acceptance.

## Findings and fixes

| Priority | Finding | Resolution | Evidence |
| --- | --- | --- | --- |
| P1 | Database production lock was generated under Python 3.13 while the container uses 3.11, omitting conditional Greenlet pinning. | Regenerated all production locks under Python 3.11 and documented the required interpreter. | Hash-enforced sequential installation and lock regression test pass. |
| P1 | A readiness test inherited CI `DATABASE_URL`, causing every generated PostgreSQL profile test job to fail. | Explicitly removed the variable inside the missing-URL test. | All three generated PostgreSQL matrices pass against PostgreSQL 16.13. |
| P1 | Documented acceptance required `db current`, but the public command did not exist. | Added the command, help text, documentation, and dispatch tests. | Real Alembic `current` and `upgrade` pass. |
| P1 | The master release workflow called a gate that rejected the master's intentional scaffold identity. | Added an explicit master marker while preserving rejection in generated projects. | Clean master gate passes; default generated identity still fails. |
| P1 | Windows Node launchers could be passed directly to CreateProcess. | Added centralized `.cmd`/`.bat` routing through `cmd.exe` across install, build, run, test, Doctor, and Tauri paths. | Cross-platform command regression tests pass; remote Windows rerun remains external. |
| P1 | An existing Tooling virtual environment was not repaired after a host Python change. | Added consistency inspection and automatic rebuild before dependency installation. | Python 3.13-to-3.14 repair reproduced successfully; regression test passes. |
| P2 | General Doctor treated absent Compose as a failure even though container tooling is optional there. | Downgraded that general-Doctor condition to a warning while retaining strict `container validate`. | Unit test and real Doctor/validation behavior pass. |

No P0 finding was identified. No P1 finding remains open in repository code.

## External items

- Push the acceptance commits and require green reruns of Core CI, Profile Matrix, PostgreSQL Integration, and Desktop CI. Until then, current-code CI is **NOT VERIFIED**.
- Run both container image builds and the local production simulation from an account with Docker daemon access. Until then, image assembly and Compose startup are **NOT VERIFIED**.
- GitHub branch protection is **NOT VERIFIED**. The public API returned HTTP 401 for the protection endpoint. Configure `main` to require a pull request, an up-to-date branch, Core CI, Profile Matrix, PostgreSQL Integration, and Desktop CI.
- Windows packaging for the accepted commit is **NOT VERIFIED** until Desktop CI reruns. macOS and Windows production signing and Apple notarization remain intentionally external product concerns.

## Deferred improvements

No repository-wide P3 feature is required for this baseline. Product repositories may later add protected signing, notarization, an updater, authentication, provider-specific deployment, business models, and product-specific browser tests when supported by concrete requirements.

## Requirement decision

| Original requirement | Status | Evidence |
| --- | --- | --- |
| Expand the reusable template | PASS | Complete profiled baseline and public workflow verified. |
| Integrate project profiles | PASS | Five practical generation, installation, test, and build paths pass. |
| Integrate optional SQL/PostgreSQL | PASS | Three valid combinations pass; two invalid combinations fail safely. |
| Extend architecture cleanly | PASS | Feature boundaries, shared contracts, lazy database access, and explicit migrations verified. |
| Improve `control.py` and tooling | PASS | Public commands, exit behavior, Windows launchers, and environment repair verified. |
| Unify configuration | PASS | Precedence, validation, derivation, and secret boundaries verified. |
| Add tests and CI | PASS WITH NOTE | Local and workflow-contract tests pass; current-code remote reruns remain external. |
| Prepare cloud and desktop builds | PASS WITH NOTE | Compose and five native Linux packages pass; current container image and Windows reruns remain external. |
| Update documentation in parallel | PASS | Commands, locks, migrations, release behavior, and this protocol are synchronized. |

## Final decision

**READY FOR CASE STUDY**

The repository is a stable, reproducible, and documented baseline for recurring web, cloud, and desktop customer projects. The decision covers repository implementation and local/native acceptance. It does not claim current-code GitHub CI, Docker image assembly, branch protection, signing, or publication as externally verified.

## Related documents

- [Framework architecture](../def/architecture.md)
- [Project profiles](../def/project-profiles.md)
- [Database feature](../def/database-feature.md)
- [Runtime configuration](../def/configuration.md)
- [Deployment architecture](../def/deployment-architecture.md)
- [Continuous Integration](../tools/ci.md)
- [Container builds and local production simulation](../tools/container-builds.md)
- [Release and desktop packaging model](../tools/release-model.md)
- [Tooling Guide](../tools/tooling.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-07 | Recorded repository-wide final technical acceptance. | Project team |
