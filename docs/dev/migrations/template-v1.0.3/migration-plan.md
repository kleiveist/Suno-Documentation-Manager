<!-- AUTO-GENERATED:backlink START -->
[← Back](template-v1.0.3.md)
<!-- AUTO-GENERATED:backlink END -->
# Template v1.0.3 Technical Migration Plan

| Field | Value |
| --- | --- |
| Status | IN PROGRESS — technical integration complete; Gate B approval required |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |

## Objective and target architecture

Align SunoDM with the reconstructed Template-Projekte `v1.0.3` scaffold at commit `8f4299193b5afe8545dda55208baf3adec23a68a` while preserving the product starting history at `507be8b6124d5d3ceb1974f299ba7722157e0980`, product version `0.1.0`, product identity, behavior, assets, SQLite contracts, file formats, and user-data boundary.

```text
TypeScript product UI
  -> product-owned typed Tauri adapter
  -> 54 registered Rust commands and managed state
  -> product application/domain services
  -> product filesystem and bundled SQLite persistence
```

The active profile remains `desktop-local` with exactly `frontend` and `tauri`. FastAPI, backend HTTP communication, cloud runtime, PostgreSQL, server migrations, and `DATABASE_URL` as a product requirement are excluded.

## Approved source-license decision

The repository stores the unmodified PolyForm Shield License 1.0.0 body in `LICENSE`. `NOTICE` contains `Required Notice: Copyright (c) 2026 Blobbite.com` and `Required Notice: Licensor: Blobbite.com`. Competing products and services are prohibited. Existing third-party terms and notices remain independently applicable.

## Immutable contracts

Gate B is required before changing any of the following:

- `.suno-doc/workspace.sqlite`, schema 7, migrations v1–v7, table/Serde semantics;
- workspace, track, evidence, archive, document, certificate, timestamp, or secret paths;
- evidence roles/provenance tokens, SHA-256 format/normalization/exclusions;
- workflow, document, manifest, certificate, PDF/A, timestamp, or audio-screening formats; or
- native command names, arguments, serialization, and frontend mappings.

No Gate B change is part of this plan.

## Migration blocks

| Block | Target and affected areas | Source/action | Expected diff | Verification | Rollback point |
| ---: | --- | --- | --- | --- | --- |
| 1 | Migration reports, `AGENTS.md`, `.gitattributes`, `.gitignore`, `LICENSE`, README license statement, product governance decision | Add reviewed reports and scaffold metadata; merge product root; use approved official license; keep unapproved community files absent | New evidence/metadata/license; narrow README/ignore merge | Full diff, license text, placeholder/secret scan, `git diff --check`; docs check may be deferred to block 7 | Reverse only block-1 patch after path list review |
| 2 | `tools/control.py`, parser, lifecycle, shared install/config/profile/Tauri tooling and tests | Copy exact generic v1.0.3 additions; three-way merge existing product adaptations | Lifecycle actions added; existing command names/product paths preserved | CLI maps, doctor/config/template help, tools suite | Reverse block-2 paths; no data touched |
| 3 | `project-profile.toml`, `VERSION`, environment/profile/shared config, runtime identity | Preserve matching identity/profile; merge only v1.0.3 fixes; scan placeholders | Product identity unchanged; no active backend/database feature | Config doctor, version check, doctor, semantic searches | Reverse block-3 paths |
| 4 | `config/code-quality.toml`, analyzer source/artifact/provenance, frontend quality configs and local test control | Add v1.0.3 quality framework; product-shape applicable frontend checks; retain tests | New quality policy/tools; no broad suppressions | Quality, tools, frontend and Tauri suites; full suite when available | Reverse block-4 additions/config merges |
| 5 | Product-owned `.github` validation workflows and dependency policy | Adapt reference jobs to `desktop-local` and actual commands; exclude publish, backend and Postgres jobs | New CI definitions only; remote status remains `NOT RUN` | Local YAML/tooling/quality validation and full diff | Reverse workflow patch |
| 6 | Web/Tauri/desktop build and release validation tooling, manifests, assets, sidecars | Merge v1.0.3 generic fixes with SunoDM names/resources; no dependency upgrade or publish | Dry-run/build validation improvements; identity unchanged | Web build, Tauri doctor, desktop dry run; platform-specific results scoped honestly | Reverse block-6 paths |
| 7 | Managed generic docs, product docs navigation, migration navigation | Add applicable lifecycle/quality docs, preserve historical/product docs, regenerate indexes with PyGitIndex | English docs/navigation; no historical result rewriting | Index dry run/apply, docs diff, docs check | Reverse docs/index patch after full docs diff |
| 8 | Cargo/npm manifests and locks, Tauri config/capability, native/frontend entry/config files | Controlled integration; retain product dependencies, all commands, identity, CSP, resources, and no HTTP backend | Minimal manifest/config/entry changes; no product feature replacement | Cargo metadata/fmt/check/clippy/test, frontend type/lint/test/build, static command parity | Reverse exact integration files; no product data touched |
| 9 | Product modules, architecture boundaries, critical integration tests | Preserve existing modules; add only characterization/integration tests needed by scaffold | No data/format changes; possible test-only additions | Workspace/SQLite/track/evidence/hash/certificate/document and IPC tests on synthetic fixtures | Reverse test-only/product-structure patch; Gate B on semantic need |
| 10 | Proven obsolete duplicate config/demo/backend artifacts, placeholder and consistency drift | Remove only items with a tested replacement; retain history, fixtures, assets, notices | Narrow cleanup; exact deliberate baseline omissions remain | Placeholder, duplicate, artifact, backend/Postgres, secret/data scans plus complete suite | Reverse each documented deletion independently |

Blocks execute in this order. There are no intermediate commits unless separately authorized.

## Current technical outcome

Blocks 1–10 are implemented on the unstaged migration tree. Identity, profile, licensing, tooling, frontend, Rust/Tauri structure, product command parity, documentation, CI configuration, build validation, and final consistency scans are complete. The quality gate, tools, frontend, E2E, web build, Tauri diagnostics, desktop dry run, Cargo metadata, formatting, and Clippy all pass.

The mandatory Rust run still has the exact pre-migration result: 383 passed, 5 failed, and 1 explicitly environment-bound test ignored. All five failures concern certificate/timestamp output or compatibility semantics. The stop rule and Gate B boundary therefore apply before any attempt to correct them. A user-authorized WIP checkpoint and branch push may preserve this unfinished state for a workstation handoff; that checkpoint is not the Gate C technical commit and does not start adoption, lifecycle state, final reporting, or a release.

## Expected deliberate lifecycle deviations

Exactly these reconstructed managed files remain absent:

```text
frontend/src/api/backend.ts
frontend/src/api/backend.test.ts
docs/atp/completed/ATP-0001-template-lifecycle.md
```

The Rust analyzer WASM is copied as an exact released artifact but is `PRODUCT_OWNED` because lifecycle manifest creation ignores `dist/` directories. This known gap must be verified by hash and provenance rather than hidden.

## Technical verification matrix

Commands are executed only when defined by the migrated tooling. Every result is recorded with its exit code; unavailable, platform-specific, remote, and manual checks are not reported as passing.

```sh
python tools/control.py doctor
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
cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
git diff --check
```

API, database, PostgreSQL, backend-container, publishing, signing, notarization, and remote GitHub runs are `NOT APPLICABLE` or `NOT RUN` for this local technical migration.

## Compatibility approach

No original database or workspace exists in the repository and no original user data may be used. The product already contains synthetic Rust fixtures and extensive unit tests. Compatibility verification will:

1. preserve hashes of existing tracked format fixtures and third-party/product assets;
2. run schema v1–v7 and current v7 persistence tests in temporary directories;
3. run legacy/current certificate, manifest, timestamp, evidence, hashing, and document tests;
4. statically recheck all 54 invoke/registration names; and
5. record GUI, cross-platform, signing, and any missing full restart fixture as manual or `NOT RUN`.

A mutating test may operate only in a new temporary directory. If a safe test cannot be isolated, the result is `MANUAL VERIFICATION REQUIRED`, not a product-workspace test.

## Stop criteria

Stop immediately on:

- any required data or format semantic change;
- a database, workspace, secret, private key, user file, or release artifact in the diff;
- active backend, cloud, database, or Postgres profile capability;
- loss or mismatch of a native command or frontend invoke;
- widened Tauri shell/filesystem permission without a product requirement;
- product version, identifier, binary, resource, sidecar, or product-name drift;
- a critical unresolved ownership decision, unexplained additional missing managed path, template repository mutation, or failed mandatory test; or
- any need to fetch, pull, push, tag, publish, sign, or modify a remote.

## Manual verification and known limitations

- Remote GitHub Actions runs require a later push and remain `NOT RUN`.
- Windows and macOS bundles, signing, notarization, and updater behavior are not provable on the local Linux host.
- Native GUI smoke/restart behavior requires an interactive session if automation is unavailable.
- A full persisted end-to-end workspace fixture may remain manual even when isolated subsystem tests pass.
- The product retains inherited warning-level size and complexity findings. The final quality run reports them without suppression and passes with zero rule errors; they are not combined with an unsafe behavioral rewrite.

## Commit and adoption boundaries

1. Gate C: after all technical and compatibility checks, show the complete unstaged diff and stop for `FREIGABE TECHNISCHER COMMIT`.
2. Technical commit: one selective commit; lifecycle state files excluded.
3. Gate D: run the read-only adoption preview only on a clean technical commit, then stop for `FREIGABE ADOPTION`.
4. Gate E: adoption may change only `.template/state.toml` and `.template/baseline.json`; stop for `FREIGABE ADOPTION COMMIT`.
5. After the pure adoption commit, create/finalize `final-report.md` and navigation.
6. Gate F: stop for `FREIGABE ABSCHLUSSCOMMIT`; do not push or release.
