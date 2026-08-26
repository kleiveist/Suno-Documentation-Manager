<!-- AUTO-GENERATED:backlink START -->
[← Back](template-v1.0.3.md)
<!-- AUTO-GENERATED:backlink END -->
# Template v1.0.3 Technical Migration Plan

| Field | Value |
| --- | --- |
| Status | Active |
| Execution status | Gate C technical commit `23c6152` exists; identity correction verified; correction commit and adoption still open |
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

## Controlled contracts and Gate B decision

The following contracts remain immutable outside an explicitly approved correction:

- `.suno-doc/workspace.sqlite`, schema 7, migrations v1–v7, table/Serde semantics;
- workspace, track, evidence, archive, document, certificate, timestamp, or secret paths;
- evidence roles/provenance tokens, SHA-256 format/normalization/exclusions;
- workflow, document, manifest, certificate, PDF/A, timestamp, or audio-screening formats; or
- native command names, arguments, serialization, and frontend mappings.

The product owner explicitly supplied `FREIGABE DATEN- ODER FORMATÄNDERUNG` on 2026-08-25. That decision adds exactly the following work to block 9:

1. preserve the disabled external-timestamp service wording in the default finalization snapshot instead of rendering `NOT DOCUMENTED`;
2. add the external-timestamp addendum statement that the application does not determine any legal qualification;
3. add the automatic timestamp-addendum PDF statement that the record does not establish a qualified timestamp, legal effect, or rights determination;
4. render the successful final-certificate binding as `Manifest hash binding VERIFIED`; and
5. emit `HASH_LIST_V1_HEADER` for sidecar format v2 while verifying already published v2-labelled lists for backward compatibility.

This correction set changes no SQLite schema or migration, workspace schema or path layout, persisted DTO, manifest schema, sidecar JSON schema, or 54-command native interface. Any additional data or format change is outside the approval and triggers a new stop.

## Migration blocks

| Block | Target and affected areas | Source/action | Expected diff | Verification | Rollback point |
| ---: | --- | --- | --- | --- | --- |
| 1 | Migration reports, `AGENTS.md`, `.gitattributes`, `.gitignore`, `LICENSE`, README license statement, product governance decision | Add reviewed reports and scaffold metadata; merge product root; use approved official license; keep unapproved community files absent | New evidence/metadata/license; narrow README/ignore merge | Full diff, license text, placeholder/secret scan, `git diff --check`; docs check may be deferred to block 7 | Reverse only block-1 patch after path list review |
| 2 | `tools/control.py`, parser, lifecycle, shared install/config/profile/Tauri tooling and tests | Copy exact generic v1.0.3 additions; three-way merge existing product adaptations | Lifecycle actions added; existing command names/product paths preserved | CLI maps, doctor/config/template help, tools suite | Reverse block-2 paths; no data touched |
| 3 | `project-profile.toml`, `VERSION`, environment/profile/shared config, runtime identity | Preserve matching identity/profile; merge only v1.0.3 fixes; scan placeholders | Product identity unchanged; no active backend/database feature | Config doctor, version check, doctor, semantic searches | Reverse block-3 paths |
| 4 | `config/code-quality.toml`, analyzer source/artifact/provenance, frontend quality configs and local test control | Add v1.0.3 quality framework; product-shape applicable frontend checks; retain tests | New quality policy/tools; no broad suppressions | Quality, tools, frontend and Tauri suites; full suite when available | Reverse block-4 additions/config merges |
| 5 | Product-owned `.github` validation workflows and dependency policy | Adapt reference jobs to `desktop-local` and actual commands; exclude publish, backend and Postgres jobs | New CI definitions only; remote status remains `NOT RUN` | Local YAML/tooling/quality validation and full diff | Reverse workflow patch |
| 6 | Web/Tauri/desktop build and release validation tooling, manifests, assets, sidecars; three release-publishing modules and their two release/community ownership tests | Merge v1.0.3 generic fixes with SunoDM names/resources; adapt the five managed release/community paths; require the three legal files in scaffold/source/web packaging; no dependency upgrade, publish workflow, or publication | Dry-run/build validation improvements, restored managed paths, and exact-SHA distribution guards; identity unchanged | Web build, Tauri doctor, desktop dry run, release/tooling tests, legal-file equality, exact-SHA binding, and workflow publication scan; platform-specific results scoped honestly | Reverse block-6 paths |
| 7 | Managed generic docs, product docs navigation, migration navigation | Add applicable lifecycle/quality docs, preserve historical/product docs, regenerate indexes with PyGitIndex | English docs/navigation; no historical result rewriting | Index dry run/apply, docs diff, docs check | Reverse docs/index patch after full docs diff |
| 8 | Cargo/npm manifests and locks, Tauri config/capability, native/frontend entry/config files | Controlled integration; retain product dependencies, all commands, identity, CSP, resources, and no HTTP backend | Minimal manifest/config/entry changes; no product feature replacement | Cargo metadata/fmt/check/clippy/test, frontend type/lint/test/build, static command parity | Reverse exact integration files; no product data touched |
| 9 | Product modules, architecture boundaries, critical integration tests, and the five Gate B certificate/timestamp corrections | Preserve existing modules; apply only the explicitly approved wording, PDF status, default snapshot, and hash-list compatibility corrections | Narrow approved data/format semantics; no SQLite/workspace schema or IPC change | Workspace/SQLite/track/evidence/hash/certificate/document/timestamp and IPC tests on synthetic fixtures | Reverse the five correction hunks independently; stop on any broader semantic need |
| 10 | Proven obsolete duplicate config/demo/backend artifacts, placeholder and consistency drift | Remove only items with a tested replacement; retain history, fixtures, assets, notices | Narrow cleanup; exact deliberate baseline omissions remain | Placeholder, duplicate, artifact, backend/Postgres, secret/data scans plus complete suite | Reverse each documented deletion independently |

Blocks execute in this order. There are no intermediate commits unless separately authorized.

## Block dependencies

| Block | Must be complete first | Completion dependency |
| ---: | --- | --- |
| 1 | Gate A | Repository evidence, identity decisions, and governance inputs are recorded before source integration. |
| 2 | Block 1 | Shared control and lifecycle commands depend on the recorded ownership boundary. |
| 3 | Blocks 1–2 | Profile and identity checks depend on the integrated command surface. |
| 4 | Blocks 2–3 | Quality configuration must evaluate the effective `desktop-local` profile and actual tools. |
| 5 | Blocks 3–4 | Product CI definitions depend on stable profile commands and local quality/test entry points. |
| 6 | Blocks 2–5 | Build and release validation depends on the command map, product identity, quality gates, and non-publishing workflow decision. |
| 7 | Blocks 1–6 | Current-state documentation depends on the settled tooling, profile, CI, and build/release shape. |
| 8 | Blocks 2–6 | Bootstrap and manifest integration depends on the stable commands, profile, and build contract. |
| 9 | Blocks 3 and 8, plus the 2026-08-25 Gate B decision for only the five named corrections | Product compatibility work depends on fixed identity/profile and the integrated frontend/Tauri/Rust boundary. |
| 10 | Blocks 1–9 | Cleanup is permitted only after every retained or replaced path has an implemented and tested owner. |

The technical full matrix depends on block 10 and the focused Gate B tests; those dependencies are satisfied. Gate C produced technical commit `23c6152`. Its first read-only adoption preview identified a display-identity mismatch, so the verified correction tree now requires a separate explicit technical correction commit. The official adoption preview depends on that later clean commit; the adoption commit depends on its own explicit approval; final-report completion depends on the pure adoption commit.

## Current technical outcome

Blocks 1–10, the five restored release/community managed paths, and the five explicitly approved Gate B corrections are present in technical commit `23c6152`. The post-commit identity correction is implemented on the unstaged tree. Identity, profile, licensing, tooling, frontend, Rust/Tauri structure, product command parity, documentation, CI configuration, build validation, and the full local technical matrix are verified. No publish workflow was added and no publication was performed.

The fixed v1.0.3 lifecycle contract resolves the master prompt's phase-20.3 identity shorthand: display-facing `name`, Tauri `productName`, window title, and frontend bootstrap are `Suno Documentation Manager`; slug, Cargo/npm package identity, `mainBinaryName`, executable, and product-owned archives remain `sunodm`. Standard Tauri bundle filenames may follow `productName`. The prior `sunodm`-derived WiX UpgradeCode is pinned explicitly so the display-name correction preserves MSI upgrade continuity; profile generation rekeys a copied pin from the target binary to prevent cross-product MSI collisions. This is an identity-metadata correction required by the exact adoption command, not a product-data or persistence-format migration.

The current complete Rust run discovers 392 tests: 391 pass, 0 fail, and 1 disposable-filesystem test is explicitly ignored because it requires an environment-provided removable-filesystem root. The five former certificate/timestamp failures are resolved without a SQLite or workspace-schema change. Technical commit `23c6152` does not start adoption, lifecycle state, final reporting, or a release. Its first read-only adoption preview stopped on the identity mismatch, and the correction remains uncommitted pending the renewed technical-commit gate.

Legal distribution hardening is active without enabling publication. Every scaffold profile receives the exact product `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md` through the `core` profile paths. Source packages are created from the exact authorized commit SHA and must include all three files; web packages must include identical copies and are validated against that exact-SHA source package. No publish workflow exists.

`.github/release-notes/` is intentionally absent and inactive because the product has no publication path. Its absence is not a technical migration gap. It may be created only after separate release authorization, together with a reviewed `.github/release-notes/<tag>.md` for the authorized tag.

## Verified technical and correction matrix

| Area | Result | Evidence |
| --- | --- | --- |
| Regenerated read-only lifecycle audit | PASS | Run `20260825T192159164542Z`; 302 operations: 211 `PRESERVE`, 88 `CONFLICT`, and exactly the three deliberate `ADD` paths; 333 product-owned paths |
| Quality | PASS | 414 files; 0 errors, 210 strong warnings, 501 warnings, and 0 suppressed findings |
| Tools | PASS | 864 passed and 25 skipped |
| Frontend | PASS | 200 tests passed |
| E2E | PASS | 2 Playwright tests passed |
| Rust | PASS | 391 passed, 0 failed, and 1 environment-bound ignored test out of 392 |
| Complete report run | PASS | `python tools/control.py test --suite all --report` completed successfully |
| Build and diagnostics | PASS | Doctor, configuration, version, documentation, web build, Tauri diagnostics, desktop dry run, Cargo metadata, formatting, and Clippy checks passed |
| Phase 24 compatibility | PASS | 16 of 16 checks passed; schema 7 remained schema 7; unexpected-change set `{}` |
| Diff formatting | PASS | `git diff --check` reports no whitespace errors |

This is a verified post-Gate-C correction result, not final migration acceptance. The identity correction commit and every adoption/finalization gate remain pending explicit approval.

## Expected deliberate lifecycle deviations

Exactly these reconstructed managed files remain absent:

```text
frontend/src/api/backend.ts
frontend/src/api/backend.test.ts
docs/atp/completed/ATP-0001-template-lifecycle.md
```

The Rust analyzer WASM is copied as an exact released artifact but is `PRODUCT_OWNED` because lifecycle manifest creation ignores `dist/` directories. This known gap must be verified by hash and provenance rather than hidden.

## Technical verification matrix

The following locally applicable commands were executed through the migrated tooling and passed. Unavailable, platform-specific, remote, and manual checks are not reported as passing.

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
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features --no-fail-fast
git diff --check
```

API, database, PostgreSQL, backend-container, publishing, signing, notarization, and remote GitHub runs are `NOT APPLICABLE` or `NOT RUN` for this local technical migration.

## Compatibility approach

No original database or workspace exists in the repository and no original user data was used. The product contains synthetic Rust fixtures and extensive unit tests. Compatibility verification:

1. preserved hashes of existing tracked format fixtures and third-party/product assets;
2. ran schema v1–v7 and current v7 persistence tests in temporary directories;
3. ran legacy/current certificate, manifest, timestamp, evidence, hashing, and document tests;
4. statically rechecked all 54 invoke/registration names;
5. exercised the 16-step close/restart/reload path in an isolated temporary workspace; and
6. records installed-GUI, cross-platform, signing, and remote checks as manual or `NOT RUN`.

A mutating test may operate only in a new temporary directory. If a safe test cannot be isolated, the result is `MANUAL VERIFICATION REQUIRED`, not a product-workspace test.

## Stop criteria

Stop immediately on:

- any data or format semantic change outside the exact five-item Gate B approval;
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
- Installed native-GUI close/restart behavior remains manual; the persisted 16-step application-service workspace fixture passes locally.
- The product retains inherited warning-level size and complexity findings. The final quality run reports them without suppression and passes with zero rule errors; they are not combined with an unsafe behavioral rewrite.

## Commit and adoption boundaries

Current state: Gate C technical commit `23c6152` exists and is also visible at `origin/migration/template-v1.0.3`. Its first read-only adoption preview failed closed on display-identity drift. The correction is verified but uncommitted; adoption has not started and `.template` remains absent.

1. Gate C completed: the reviewed technical tree became commit `23c6152`; lifecycle state files were excluded.
2. Correction Gate C: show the complete identity-correction diff and stop again for `FREIGABE TECHNISCHER COMMIT`; create a separate local correction commit because the original commit is already visible on the remote-tracking branch and must not be rewritten.
3. Gate D: run the read-only adoption preview only on the clean corrected commit, then stop for `FREIGABE ADOPTION`.
4. Gate E: adoption may change only `.template/state.toml` and `.template/baseline.json`; stop for `FREIGABE ADOPTION COMMIT`.
5. After the pure adoption commit, create/finalize `final-report.md` and navigation.
6. Gate F: stop for `FREIGABE ABSCHLUSSCOMMIT`; do not push or release.
