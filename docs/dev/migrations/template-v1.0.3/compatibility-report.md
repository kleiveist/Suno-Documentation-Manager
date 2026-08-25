<!-- AUTO-GENERATED:backlink START -->
[← Back](template-v1.0.3.md)
<!-- AUTO-GENERATED:backlink END -->
# Template v1.0.3 Compatibility Report

| Field | Value |
| --- | --- |
| Status | FAIL — five characterized certificate/timestamp requirements need Gate B approval |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Product baseline | `507be8b6124d5d3ceb1974f299ba7722157e0980` |
| Template baseline | `8f4299193b5afe8545dda55208baf3adec23a68a` (`v1.0.3`) |

## Verification boundary

Compatibility checks use only repository-owned synthetic fixtures, in-memory state, and newly created temporary directories. No original workspace, SQLite database, track directory, evidence file, certificate, user configuration, backup, signing material, or other user data is opened or transformed.

The migration deliberately preserves the following product contracts:

- product version `0.1.0`, profile `desktop-local`, and exactly the `frontend` and `tauri` features;
- `.suno-doc/workspace.sqlite`, SQLite schema 7, and migrations v1–v7;
- workspace, track, evidence, archive, document, certificate, and timestamp paths;
- evidence roles, persisted Serde tokens, SHA-256 rules, exclusions, and normalization;
- document template `1.11`, certificate/PDF format `6.2`, manifest schema `9`, and timestamp sidecar formats; and
- all 54 TypeScript invoke names and registered Rust commands.

Any change to those data or format semantics remains outside this migration and requires Gate B approval.

## Pre-migration characterization

Before product-module restructuring, the complete Rust test binary ran against the unchanged product source. The characterization result was 383 passed, 5 failed, and 1 ignored. The five failures were all in existing certificate/timestamp expectations:

1. end-to-end portable certificate generation;
2. application external-timestamp addenda;
3. automatic timestamp-addendum PDF rendering;
4. final certificate PDF timestamp qualification; and
5. published external-timestamp byte verification.

These failures are not reported as migration regressions, but they remain mandatory-test failures until their causes are resolved or an explicitly approved data/format change is performed. Tests have not been disabled, ignored, or weakened to conceal them.

## Structural compatibility evidence

| Area | Current evidence | Status |
| --- | --- | --- |
| TypeScript/Tauri command contract | Static extraction found the same 54 invoke names, Rust command functions, and handler registrations | PASS |
| Rust model | 97 public items preserved; Serde/default/wire values and ordered string inventory preserved | PASS |
| SQLite persistence | Public API and ordered SQL/string inventory preserved; schema and migrations unchanged | PASS |
| Workflow | Public exports, embedded workflow source, requirement rules, ordering, and test inventory preserved | PASS |
| Frontend workflow | 27 exports, 575 ordered strings, and all 49 original test ASTs preserved | PASS |
| Translation catalogs | 2,181 entries, tuple order, and catalog hashes preserved | PASS |
| Certificate PDF | 17 public items and 2,450 string literals preserved; seven representative before/after PDFs are byte-identical | PASS |
| Product UI | Product bootstrap and adapters preserved; all 54 native calls remain mapped; 200 frontend tests and two Playwright tests pass | PASS |
| Application service | Public API and ordered literal inventory preserved; focused run reproduced exactly the same two application failures as the pre-migration binary and passed the other 98 tests | PASS AGAINST BASELINE |
| Certificate renderer | Public API and ordered literal inventory preserved; 34 focused tests pass and eight representative synthetic artifact sets remain byte-identical | PASS |
| Document renderer | Nine public API items and 726 ordered literals preserved; 19 focused tests pass and all 16 standard/multi-track fixture files remain byte-identical | PASS |
| Audio screening | Public behavior and 592 ordered literals preserved; 24 focused tests pass and representative WAV, JSON, and Markdown bytes remain identical | PASS |
| External timestamps | Public API and 599 ordered literals preserved; 23 of 24 focused tests pass and OTS/provider-response fixture bytes remain identical; the one failure is the characterized published-hash-list requirement | FAIL — GATE B |

## Technical command matrix

The matrix below was run on the stable, unstaged technical tree. Warnings reported by the quality analyzer are visible, unsuppressed inherited findings; its policy gate has zero errors. A failed mandatory command keeps the overall result at `FAIL` even where the same failure was proven to predate the migration.

| Command | Exit code | Status | Relevant notes |
| --- | ---: | --- | --- |
| `python tools/control.py doctor` | 0 | PASS | Effective `desktop-local` configuration is valid; backend runtime is disabled |
| `python tools/control.py config doctor` | 0 | PASS | Effective configuration is valid |
| `python tools/control.py quality` | 0 | PASS | 407 files; 0 errors, 210 strong warnings, 497 warnings, 0 suppressed findings; all language/tool gates pass |
| `python tools/control.py test --suite tools` | 0 | PASS | Restored tooling suite passes; the real AST check asserts the SunoDM desktop-adapter edge rather than the deliberately absent backend adapter |
| `python tools/control.py test --suite frontend` | 0 | PASS | 200 tests pass; coverage is 90.74% statements, 82.74% branches, 87.72% functions, and 92.71% lines |
| `python tools/control.py test --suite tauri` | 1 | FAIL | Cargo check passes; Rust result is 383 passed, 5 failed, 1 ignored, exactly matching pre-migration characterization |
| `python tools/control.py test --suite e2e` | 0 | PASS | Two Playwright tests pass; runner-owned services start and stop cleanly |
| `python tools/control.py test --suite all --report` | 1 | FAIL | Tools, schema, frontend, and E2E pass; API/database/PostgreSQL skip as disabled; Tauri fails only on the five characterized cases |
| `python tools/control.py docs check` | 0 | PASS | Regenerated navigation is consistent across 66 pages |
| `python tools/control.py version check` | 0 | PASS | All seven version sources are `0.1.0` |
| `python tools/control.py build web` | 0 | PASS | Web build and ignored local ZIP generation succeed |
| `python tools/control.py tauri doctor` | 0 | PASS | Overall `WARN`: optional pnpm/Corepack and AppImage packaging helpers are absent; required Tauri/GTK/WebKit dependencies are present |
| `python tools/control.py build desktop --dry-run --no-clean` | 0 | PASS | Validates `deb,rpm,appimage` command composition; creates no desktop artifacts |
| `cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps` | 0 | PASS | One package/workspace member, two targets, 25 dependencies |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 | PASS | No formatting drift |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | 0 | PASS | No warnings or errors |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features` | 101 | FAIL | 389 tests: 383 passed, 5 failed, 1 ignored; the ignored disposable-filesystem test requires an explicit removable-filesystem root |
| `git diff --check` | 0 | PASS | No whitespace errors |

The API, database, PostgreSQL, and backend-container suites are `NOT APPLICABLE` to `desktop-local`. Remote GitHub Actions, macOS and Windows package execution, signing, notarization, updater delivery, publishing, and release creation are `NOT RUN` in this local migration.

## Product and data compatibility

| Operation | Isolated evidence | Result |
| --- | --- | --- |
| Create/open a temporary workspace and initialize SQLite | Application and persistence fixtures in the passing Rust set | PASS |
| Load schema 7 and exercise migrations v1–v7 | Persistence migration fixtures in the passing Rust set; schema and ordered SQL inventory unchanged | PASS |
| Create, update, classify, reload, and revise tracks | Application/model/workflow fixtures in the passing Rust set | PASS |
| Import, replace, remove, and verify evidence | Temporary-file evidence/application fixtures in the passing Rust set | PASS |
| Generate and verify SHA-256 inventories | Integrity/application fixtures in the passing Rust set; hash algorithms and exclusions unchanged | PASS |
| Generate and regenerate managed documents | 19 focused tests; 16 of 16 representative files remain byte-identical | PASS |
| Generate and verify certificate artifacts | 34 focused renderer tests and representative artifact equality pass; three mandatory certificate/output expectations still fail exactly as before migration | FAIL — GATE B |
| Create, publish, reload, and verify timestamp sidecars | 23 of 24 focused tests pass; OTS proof and provider-response fixture bytes remain identical | FAIL — GATE B |
| Run local/external audio-screening fixtures | 24 focused tests pass; representative WAV/JSON/Markdown bytes remain identical | PASS |
| Close/reopen subsystem state | Isolated persistence/application reload fixtures pass | PASS |
| Close/reopen an installed native GUI on all supported platforms | No safe repository-owned installed-application fixture exists | MANUAL VERIFICATION REQUIRED |
| Confirm original product data was untouched | Final status and path scan finds no workspace, SQLite, WAL/SHM, user file, secret, or private key in the migration diff | PASS |

No repository-owned persisted end-to-end workspace fixture exists. Native GUI restart/reload, an installed desktop bundle on each supported operating system, and signing/updater behavior therefore remain manual or platform-specific verification rather than locally claimed passes.

## Gate B findings

The following requirements fail both the preserved pre-migration binary and the migrated tree. Correcting them would alter certificate, PDF, addendum, published hash-list, snapshot, or timestamp-verification semantics, so no correction is made without the exact Gate B authorization.

1. Deterministic portable-certificate reproduction differs at byte 7,497 / line 162 between `External timestamp service is disabled.` and `NOT DOCUMENTED`.
2. The external-timestamp addendum lacks `does not determine any legal qualification`.
3. The automatic timestamp-addendum PDF lacks the normalized statement `do not establish a qualified timestamp, legal effect`.
4. The final certificate PDF lacks `Manifest hash binding VERIFIED`.
5. Published timestamp verification does not accept the expected `HASH_LIST_V1_HEADER` byte contract.

## Remaining risks

- The five pre-existing Rust certificate/timestamp failures keep the mandatory matrix at `FAIL`; resolving them requires the separately approved Gate B data/format work described above.
- The bundled `fpcalc` sidecars are not distribution-ready until the corresponding-source, relinking, and exact FFmpeg/FFT component obligations are reviewed.
- Remote CI and cross-platform bundle behavior cannot be inferred from local YAML validation or a Linux dry run.
- Interactive native GUI close/restart/reload behavior requires a later manual installed-application check if no isolated automated fixture covers it.

Technical integration is otherwise complete, but this document is not final acceptance. A user-authorized WIP checkpoint may be committed and pushed solely to transfer the unfinished migration branch between workstations. It is not the Gate C technical commit and does not authorize lifecycle adoption, a final report, a tag, signing, publishing, or a release while Gate B and the later commit/adoption gates remain outstanding.
