<!-- AUTO-GENERATED:backlink START -->
[← Back](tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Continuous integration

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Audience | Contributors and repository maintainers |
| Related ATP | Product-specific acceptance remains under [Active acceptance](../atp/active/active.md) |
| Remote execution | NOT RUN for the migration branch |

## Purpose

This document defines the product-owned GitHub Actions validation model for Suno Documentation Manager. The workflows are intentionally limited to the active `desktop-local` product: frontend, Tauri, Rust, shared tooling, documentation, security analysis, unsigned desktop candidates, and non-publishing release validation.

The workflow files are checked-in configuration only. No remote run for the migration branch has been observed or claimed, so remote CI remains `NOT RUN` until an exact commit and its actual GitHub results are recorded.

## Product scope

SunoDM enables exactly `frontend` and `tauri`. CI does not add or validate a FastAPI service, PostgreSQL, Alembic, cloud deployment, Docker product image, profile-generation matrix, signing pipeline, or publication workflow.

GitHub Actions uses the same public entry point as local development:

```text
GitHub Actions
      |
      v
python tools/control.py
      |
      +-- install / doctor / config doctor / tauri doctor
      +-- quality
      +-- test --suite tools|frontend|tauri|e2e|all
      +-- build web / build desktop
      +-- version check / release check
      +-- docs check
```

## Active workflow inventory

| Workflow | File | Pull request | Push to `main` | Manual | Weekly | Version tag | Reusable |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Core CI | `.github/workflows/ci.yml` | Yes | Yes | Yes | No | No | No |
| Desktop CI | `.github/workflows/desktop.yml` | Yes | Yes | Yes | No | No | Yes |
| Security | `.github/workflows/security.yml` | Yes | Yes | Yes | Yes | No | No |
| Release Validation | `.github/workflows/release.yml` | No | No | Yes | No | `v*.*.*` | No |

There is no `profiles.yml`, `postgres.yml`, `release-publish.yml`, signing job, deployment job, or package-registry publication path in this product.

## Core CI

`.github/workflows/ci.yml` runs one governance job followed by five independent product jobs:

| Check name | Responsibility |
| --- | --- |
| `Core / Code Quality & Architecture` | Central policy, source metrics, architecture rules, pinned Rust/WASI analyzer verification, Ruff, frontend quality tools, rustfmt, Clippy, and Cargo checks |
| `Core / Tooling & Configuration` | CLI, profile, lifecycle, configuration, and shared-tool regressions for the active product shape |
| `Core / Documentation Check` | Read-only generated-navigation validation and focused documentation-tool tests |
| `Core / Frontend Tests & Web Build` | Frontend tests, coverage controls, and the budgeted production Vite build |
| `Core / Browser Smoke & Accessibility` | Chromium smoke behavior and axe accessibility checks against the locally started frontend |
| `Core / Rust 1.88 MSRV` | Locked native-target checking at the crate's declared minimum Rust version |

The downstream jobs depend on Quality. A warning remains visible according to `config/code-quality.toml`; an unsuppressed error or failed required tool blocks the workflow. In particular, the 900-code-line hard limit is not bypassed in CI.

## Desktop CI

`.github/workflows/desktop.yml` uses native Ubuntu, macOS, and Windows runners. Each job installs the portable tooling and frontend dependencies, verifies the checked-in analyzer runtime, runs Tauri diagnostics and native tests, and builds technically available unsigned packages.

| Runner | Target | Uploaded artifact | Archive transport |
| --- | --- | --- | --- |
| Ubuntu | Linux | `sunodm-desktop-linux-unsigned` | `sunodm-desktop-linux-unsigned.tar.gz` |
| macOS | macOS | `sunodm-desktop-macos-unsigned` | `sunodm-desktop-macos-unsigned.tar.gz` |
| Windows | Windows | `sunodm-desktop-windows-unsigned` | `sunodm-desktop-windows-unsigned.zip` |

Linux defaults to an unsigned DEB. A reusable or manual call can request a validated comma-separated Linux bundle list. Release Validation requests DEB, RPM, and AppImage. The POSIX prearchives preserve executable modes before GitHub artifact upload; the Windows archive is validated before upload. Artifacts are retained for 14 days and are neither signed installers nor published releases.

## Security

`.github/workflows/security.yml` runs CodeQL `security-extended` analysis for Python, JavaScript/TypeScript, and Rust on pull requests, pushes to `main`, weekly schedule, and manual dispatch. A pull-request-only Dependency Review job rejects newly introduced high or critical vulnerabilities.

All external Actions are pinned to full commit SHAs. `.github/dependabot.yml` covers GitHub Actions, frontend npm, tooling pip, product Cargo, and analyzer Cargo inputs. Routine minor and patch updates are grouped; major updates remain separate and receive the supported review cooldown where configured. Dependabot configuration does not itself prove that an update is safe.

## Release validation

`.github/workflows/release.yml` runs by manual dispatch or a tag matching `v*.*.*`. Its validation job creates an SPDX JSON dependency SBOM, rebuild-checks the pinned analyzer, runs release-strict quality, documentation checks, the complete applicable product suite, the web build, and `release check`. It uploads short-lived web/SBOM validation evidence, then calls the same unsigned desktop workflow with Linux DEB, RPM, and AppImage targets.

The workflow has no signing, notarization, attestation, GitHub Release creation, publication, deployment, or write-capable repository job. A successful validation run would be technical evidence only; it would not by itself authorize distribution.

## Runtime baselines

| Runtime | Workflow baseline |
| --- | --- |
| Python | 3.11 |
| Node.js | 24 |
| Product Rust MSRV | 1.88.0 |
| Quality and desktop Rust | 1.97.1 |

The product crate remains authoritative for its MSRV. The pinned quality-analyzer provenance remains authoritative for the analyzer toolchain and artifact hash.

## Installation, caching, and permissions

Frontend jobs use the committed lockfile. Rust jobs use locked Cargo dependencies. GitHub-hosted caches cover npm, pip, and Cargo inputs by their relevant lock or requirement files; caches are an optimization, not evidence.

Every workflow begins with `contents: read`. CodeQL narrows `security-events: write` to its analysis job. No normal job reads signing, notarization, updater, deployment, database, or publication credentials. Pull-request concurrency cancels an older run for the same pull request; pushes to `main` and release validation are not cancelled by a later run.

## Evidence semantics

- `PASS`: the named check was actually executed for an identified commit and succeeded.
- `SKIP`: the active profile excludes the optional suite, or a documented optional condition does not apply.
- `FAIL`: a required enabled check ran and failed, or required configuration is incomplete.
- `NOT RUN`: a workflow or manual check has not been executed or no result was inspected.
- `NOT APPLICABLE`: the product architecture excludes the capability.

Workflow source, a test definition, or an inherited upstream run is not a SunoDM `PASS`. Remote CI stays `NOT RUN` for this migration until exact-SHA results exist.

## Branch protection recommendation

Configure required checks only after the product workflows have completed stable runs and GitHub exposes their actual check names. Candidate checks are the six `Core / ...` jobs, the three `Desktop / ... / Unsigned Verification` jobs, and the applicable Security checks. Require current pull-request results and do not grant validation workflows deployment or bypass privileges.

## Local verification

Run applicable checks from the repository root:

```sh
python tools/control.py config doctor
python tools/control.py quality
python tools/control.py test --suite tools
python tools/control.py test --suite frontend
python tools/control.py test --suite tauri
python tools/control.py test --suite e2e
python tools/control.py test --suite all --report
python tools/control.py docs check
python tools/control.py version check
python tools/control.py build web
python tools/control.py tauri doctor
python tools/control.py build desktop --dry-run --no-clean
```

Local execution cannot prove Windows or macOS packaging and does not substitute for a recorded remote run.

## Related documents

- [Tooling guide](tooling.md)
- [Code quality](../def/code-quality.md)
- [Application architecture](../def/architecture.md)
- [Template lifecycle](../def/template-lifecycle.md)
- [Release model](release-model.md)
- [ATP workflow](../atp/README.md)
