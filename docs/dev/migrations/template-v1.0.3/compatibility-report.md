<!-- AUTO-GENERATED:backlink START -->
[← Back](template-v1.0.3.md)
<!-- AUTO-GENERATED:backlink END -->
# Template v1.0.3 Compatibility Report

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Execution state | Technical and product/data compatibility verified; Gate C approval is pending |
| Compatibility result | PASS |
| Product baseline | `507be8b6124d5d3ceb1974f299ba7722157e0980` |
| Template baseline | `8f4299193b5afe8545dda55208baf3adec23a68a` (`v1.0.3`) |
| Gate B | Approved on 2026-08-25 |

## Purpose

This report records the technical, product, and data compatibility evidence for integrating Template-Projekte `v1.0.3` into SunoDM `0.1.0`. It describes the verified technical tree immediately before Gate C. It is not lifecycle adoption evidence or final migration acceptance.

## Scope

The applicable product profile is `desktop-local` with exactly the `frontend` and `tauri` features. FastAPI, PostgreSQL, API, database-service, and backend-container capabilities are outside this profile. The report covers the local tooling, frontend, Tauri/Rust application, product-owned SQLite persistence, workspace files, tracks, evidence, integrity lists, generated documents, certificates, and external-timestamp sidecars.

## Verification boundary

Compatibility checks use only repository-owned synthetic fixtures, in-memory state, and newly created temporary directories. No original workspace, SQLite database, track directory, evidence file, certificate, user configuration, backup, signing material, or other user data is opened or transformed.

The migration preserves these product contracts:

- product version `0.1.0`, profile `desktop-local`, and exactly the `frontend` and `tauri` features;
- `.suno-doc/workspace.sqlite`, SQLite schema 7, and migrations v1–v7;
- workspace, track, evidence, archive, document, certificate, and timestamp paths;
- evidence roles, persisted Serde tokens, SHA-256 rules, exclusions, and normalization;
- document template `1.11`, certificate/PDF format `6.2`, manifest schema `9`, and sidecar JSON format versions; and
- all 54 TypeScript invoke names and registered Rust commands.

Gate B authorized the five narrowly scoped certificate/timestamp corrections documented below. No SQLite schema, existing SQLite migration, track format, evidence format, hash algorithm, hash normalization, workspace path, signature method, or user-data migration changed. Any further data or format change remains outside this migration and requires separate approval.

## Pre-migration characterization

Before product-module restructuring, the complete Rust test binary ran against the unchanged product source. The characterization result was 383 passed, 5 failed, and 1 ignored. The five failures were all in existing certificate/timestamp expectations:

1. end-to-end portable certificate generation;
2. application external-timestamp addenda;
3. automatic timestamp-addendum PDF rendering;
4. final certificate PDF timestamp qualification; and
5. published external-timestamp byte verification.

The investigation found no evidence that a rollback on the migration branch caused these failures. All five reproduced at product baseline `507be8b6124d5d3ceb1974f299ba7722157e0980`; source-history comparison traces the regressed behavior to `0e940be`, before the Template `v1.0.3` checkpoint and module split. The migration did not introduce them. Gate B subsequently authorized their correction. Tests were not disabled, ignored, or weakened.

## Approved Gate B corrections

The exact authorization `FREIGABE DATEN- ODER FORMATÄNDERUNG` was received on 2026-08-25. The implementation is limited to the following five findings.

| Finding | Approved correction | Verification | Status |
| --- | --- | --- | --- |
| A missing finalization timestamp snapshot rendered `NOT DOCUMENTED` instead of the deterministic disabled-provider state | Define the snapshot default as provider `Disabled`, configuration `Disabled`, message `External timestamp service is disabled.`, automatic request `false`, and technical status `NotRecorded` | `end_to_end_documentation_workflow_creates_portable_certificate` | PASS |
| The external-timestamp disclaimer did not explicitly reject legal qualification inference | State that SunoDM does not determine any legal qualification of the timestamp or infer legal effect | `external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision` | PASS |
| The automatic addendum PDF lacked the normalized limitation statement | State that provider-derived metadata and technical checks do not establish a qualified timestamp, legal effect, or a rights determination | `automatic_timestamp_addendum_renders_provider_metadata_without_user_or_legal_claims` | PASS |
| The final PDF represented the provider digest match as a generic yes/no value | Render `VERIFIED`, `NOT VERIFIED`, or `NOT CHECKED` for manifest hash binding | `final_pdf_separates_concrete_timestamp_from_provider_qualification`; all three branches in `final_pdf_renders_all_manifest_hash_binding_states` | PASS |
| Sidecar JSON v2 incorrectly changed the stable hash-list header contract | Emit the stable v1 hash-list header for sidecar versions 1 and 2 while continuing to accept the briefly emitted v2-labelled header | `verification_pins_published_bytes_and_never_requires_current_renderer_output` | PASS |

The complete Rust run after these corrections and compatibility regressions discovered 392 tests: 391 passed, none failed, and one explicitly environment-bound removable-filesystem test remained ignored.

## Structural compatibility evidence

| Area | Current evidence | Status |
| --- | --- | --- |
| Product identity and profile | Product version remains `0.1.0`; profile resolves to `desktop-local` with `frontend` and `tauri`; slug, binary, bundle identifier, and display name remain product-specific | PASS |
| Frontend/Tauri command boundary | The 54 invoke names remain mapped to registered native commands; no FastAPI transport was introduced | PASS |
| Rust application architecture | Module splits retain application, domain, persistence, rendering, integrity, and adapter responsibilities; the complete Rust suite passes | PASS |
| SQLite persistence | Database path, schema 7, migrations v1–v7, typed repositories, and transaction behavior are unchanged | PASS |
| Workspace, track, and evidence | Existing relative paths, roles, persisted tokens, validation rules, and SHA-256 evidence bindings are preserved | PASS |
| Integrity | SHA-256 algorithm, normalization, candidate set, exclusions, and verification rules are unchanged | PASS |
| Documents and certificates | Current document, manifest, and certificate version identifiers remain unchanged; deterministic regeneration and certificate verification pass | PASS |
| External timestamps | The five approved presentation/header corrections pass, and the v2-labelled sidecar header remains readable for backward compatibility | PASS |
| Product UI | Product bootstrap and desktop adapters remain connected; 200 frontend tests and two Playwright tests pass | PASS |
| Release/tooling integration | Product-scoped tooling tests pass without introducing backend/PostgreSQL profile behavior or performing a remote release action | PASS |

## Distribution hardening and inactive publication

`profiles/features.toml` includes `LICENSE`, `NOTICE`, and `THIRD_PARTY_NOTICES.md` in the `core` paths used by every scaffold profile. The scaffold tests require exact byte equality with the product files. Source packages are created from the exact authorized commit SHA and must contain all three legal files; web packages must contain the same three files byte-for-byte and are validated against that exact-SHA source package.

These are fail-closed distribution controls, not publication enablement. No publish workflow exists in the product. `.github/release-notes/` is intentionally absent and inactive, and its absence is not a technical migration gap. Only a separate release authorization may introduce the directory together with a specifically reviewed `.github/release-notes/<tag>.md` for the authorized tag.

## Technical command matrix

The matrix below was run on the stable pre-Gate-C technical tree. Quality warnings remain visible and unsuppressed; the configured gate reports zero errors.

| Command | Exit code | Status | Relevant notes |
| --- | ---: | --- | --- |
| `python tools/control.py doctor` | 0 | PASS | Effective `desktop-local` configuration is valid; backend runtime is disabled |
| `python tools/control.py config doctor` | 0 | PASS | Effective configuration is valid |
| `python tools/control.py quality` | 0 | PASS | 414 files; 0 errors, 210 strong warnings, 501 warnings, 0 suppressed findings |
| `python tools/control.py test --suite tools` | 0 | PASS | 863 passed and 25 skipped |
| `python tools/control.py test --suite frontend` | 0 | PASS | 200 tests pass; coverage is 90.74% statements, 82.74% branches, 87.72% functions, and 92.71% lines |
| `python tools/control.py test --suite tauri` | 0 | PASS | Rust result is 391 passed, 0 failed, and 1 environment-bound test ignored |
| `python tools/control.py test --suite e2e` | 0 | PASS | Two Playwright tests pass; runner-owned services start and stop cleanly |
| `python tools/control.py test --suite all --report` | 0 | PASS | `.report/test-report-20260825-200936-suite-all-ok.md` records tools, schema, frontend, Tauri, and E2E as OK; API, database, and PostgreSQL are skipped because their features are disabled |
| `python tools/control.py docs check` | 0 | PASS | Documentation navigation and authored-page checks are consistent |
| `python tools/control.py version check` | 0 | PASS | All seven version sources are `0.1.0` |
| `python tools/control.py build web` | 0 | PASS | Production frontend build succeeds |
| `python tools/control.py tauri doctor` | 0 | PASS | Overall `WARN` only because optional Corepack is absent; pnpm and all required Tauri, GTK, WebKit, and AppImage tools are present |
| `python tools/control.py build desktop --dry-run --no-clean` | 0 | PASS | Validates `deb,rpm,appimage` command composition; creates no desktop artifacts |
| `cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps` | 0 | PASS | One package/workspace member, two targets, 25 dependencies |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 | PASS | No formatting drift |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | 0 | PASS | No warnings or errors |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features --no-fail-fast` | 0 | PASS | 392 discovered: 391 passed, 0 failed, and 1 ignored; the ignored disposable-filesystem test requires an explicit removable-filesystem root |
| `git diff --check` | 0 | PASS | No whitespace errors |

The API, database, PostgreSQL, and backend-container suites are `NOT APPLICABLE` to `desktop-local`.

## Phase 24 product and data compatibility

The focused test `phase_24_workspace_round_trip_classifies_tree_hash_changes` executes the required sequence in a newly created temporary directory. It uses only generated WAV evidence, a synthetic profile, and a synthetic workspace. The temporary directory is removed by the test harness.

| Step | Verification | Evidence | Status |
| ---: | --- | --- | --- |
| 1 | Capture before hashes | Ordered SHA-256 maps are captured for every regular file in the source fixtures, portable track, and temporary workspace | PASS |
| 2 | Open the test workspace | `WorkspaceApp::open(..., false)` opens the prepared isolated workspace | PASS |
| 3 | Detect existing tracks | The prepared Track ID is returned by the typed SQLite repository without refreshing observation timestamps | PASS |
| 4 | Load SQLite data | The typed track record loads, `PRAGMA user_version` remains 7, and the normalized logical database is identical immediately after open/read | PASS |
| 5 | Detect evidence files | Every evidence record loads and each managed file matches its stored SHA-256 | PASS |
| 6 | Generate documents | All eight managed documents regenerate from loaded persisted state with the same pre-existing bytes and no hidden database write | PASS |
| 7 | Verify existing hashes | The existing `SHA256SUMS.txt` verifies after deterministic regeneration | PASS |
| 8 | Generate or verify certificates | Finalization creates and verifies the manifest, Markdown certificate, certificate hash list, and both language PDFs | PASS |
| 9 | Perform a controlled fixture write | A synthetic workspace-profile value changes; the finalized track snapshot remains immutable | PASS |
| 10 | Close cleanly | The first application object is dropped before restart | PASS |
| 11 | Restart | A new application object is created | PASS |
| 12 | Reopen the same workspace | The reopened root equals the original temporary workspace root | PASS |
| 13 | Reload data | Finalized track, controlled profile, evidence, normalized SQLite state, certificate, and main hashes reload and verify exactly | PASS |
| 14 | Capture after hashes | The same three ordered regular-file SHA-256 maps and normalized logical database snapshot are captured after restart | PASS |
| 15 | Classify changes | Every changed regular-file path is assigned to expected output, permitted fixture metadata, or unexpected product data | PASS |
| 16 | Exclude unintended migration | Every user-defined SQLite schema object, table, column definition, and normalized row is checked; only the exact finalization and controlled profile deltas are allowed, while all other database rows, source fixtures, evidence records, and pre-existing portable files remain unchanged | PASS |

### Focused run hashes

These are the exact values from the final focused run on 2026-08-25.

| Hash set or file | Before SHA-256 | After SHA-256 | Classification |
| --- | --- | --- | --- |
| Synthetic source-fixture regular-file set | `a140728ddb7b87a78d96a8e51762f741cbbc907faf93c314d7f8cf0459a3930a` | `a140728ddb7b87a78d96a8e51762f741cbbc907faf93c314d7f8cf0459a3930a` | Unchanged |
| Portable track regular-file set | `6cbed954576e6460204f136158d30bdfb22a4700d0ff23b7afa034e99c19c960` | `c18c2ae7386974872cc621f0894303576b2a7e6252e06fb2bd5f3f3cc9ddbfd4` | Expected certificate output added; every pre-existing regular file remains unchanged |
| Temporary workspace regular-file set | `589ff8e3fd68381ac94e8ad1e111ec9498a2b1c15e0836419ece443daa18d1d7` | `79e6b8f0cf4d4d14b4528e2247487d1ca546972f68b324c6d3908654ff7c0a81` | Expected output plus the permitted SQLite fixture change |
| `.suno-doc/workspace.sqlite` file bytes | `1b566fdb45cd4a1763e01a31938634ce5a3ec57d7c9240e404b2b8087a0df63f` | `b9139b311d8665154a343060a0c35b617bd6b915b5fe8a9ed83dfdcc0161f998` | Permitted fixture database change; schema remains 7 |
| Normalized logical SQLite snapshot | `191782480e54f2ba55b0bfced5dd8bf3afa7d2f686747de4f12963a631292366` | `c05c3997c9784cf26967c557cc3f7ec9754ad824c1b4b07a55d06593c6beaf6b` | Exact finalization and controlled-profile whitelist; every other schema object/table/row/column is unchanged |

Each aggregate regular-file digest is SHA-256 over the ordered relative-path/file-digest map; directory and symlink entries are not described as file hashes. The normalized SQLite digest covers the ordered `sqlite_schema` inventory and SQL for every user-defined table, index, trigger, or view, plus every table's ordered column contract and canonically ordered logical rows, with JSON values key-normalized before hashing. Schema 7 currently contains four user-defined indexes and no user-defined triggers or views; any addition, removal, or SQL change would fail the exact snapshot comparison. A dedicated regression creates a valid user object named `sqliteX_user_defined` and proves that only SQLite's literal reserved `sqlite_` prefix is excluded from this inventory. Certificate IDs, timestamps, and SQLite page bytes are created during the run, so the track, workspace, certificate, physical database, and normalized logical database digests are run-specific evidence rather than hard-coded golden values.

### Expected generated artifacts

| Artifact | SHA-256 from the focused run | Classification |
| --- | --- | --- |
| `06_CERTIFICATE/CERTIFICATE_SHA256.txt` | `65249c718a8382692182b8c5334575e75c9b6fc38187136dfbe8da69d10bce38` | Expected test output |
| `06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md` | `0b3e66e2e6fbe9773c6eeb61d2fb0be96c40239b87b1e7b8eaad30dc16c429e3` | Expected test output |
| `06_CERTIFICATE/EVIDENCE_MANIFEST.json` | `6157f56b0b48807c9f2615cac4ef294c3423ee1ca1d84cf40b5f30dfd8c2337f` | Expected test output |
| `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` | `b86a39ac92d49e140b16aa4ae52c34f4eca9ffffb548e5cfdbdf32b18dce1322` | Expected test output |
| `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf` | `be79c5d3a607a42fefb046225380bad0ac7713f6d34fd669152d395d4a0502ad` | Expected test output |

### Change classification

| Class | Result |
| --- | --- |
| Expected test output | Exactly the five certificate artifacts listed above |
| Permitted fixture metadata change | Only `.suno-doc/workspace.sqlite`; its normalized logical delta is restricted to the exact finalized track/certificate state, finalization timestamp summary, and controlled synthetic profile update |
| Unexpected product-data change | Empty set `{}` |

### Tracked RFC 3161 fixture preservation

The following SHA-256 values cover the stored `.b64` file bytes. The tests consume these tracked fixtures through `include_str!`. Each value is identical at product baseline `507be8b6124d5d3ceb1974f299ba7722157e0980` and in the verified technical tree.

| Tracked fixture | Baseline SHA-256 | Current SHA-256 | Status |
| --- | --- | --- | --- |
| `src-tauri/testdata/rfc3161_payload.b64` | `c52500fd549cc661e5891e80934ed5b17401d4bfe1e9f4b0919f92225e346d1e` | `c52500fd549cc661e5891e80934ed5b17401d4bfe1e9f4b0919f92225e346d1e` | PASS |
| `src-tauri/testdata/rfc3161_valid.tsr.b64` | `d3b46fd2896121777ed8421222e86151671fc3a39b131507c06ef0b6aae5cf5d` | `d3b46fd2896121777ed8421222e86151671fc3a39b131507c06ef0b6aae5cf5d` | PASS |
| `src-tauri/testdata/rfc3161_root.der.b64` | `2b7ea2fe1428194ef6c3c03e1cedddbbe8c45ec2ee5ccb9a7e55b0c4c7d89f34` | `2b7ea2fe1428194ef6c3c03e1cedddbbe8c45ec2ee5ccb9a7e55b0c4c7d89f34` | PASS |
| `src-tauri/testdata/rfc3161_rsa_valid.tsr.b64` | `f9d74c729efbc48eb6e94b4acd705aff8d90fd68a8210b9867f91e690ef8ce83` | `f9d74c729efbc48eb6e94b4acd705aff8d90fd68a8210b9867f91e690ef8ce83` | PASS |
| `src-tauri/testdata/rfc3161_rsa_root.der.b64` | `c5565914a049eb8e6c652368288dd55851948514cf5a84336d162017c3548d39` | `c5565914a049eb8e6c652368288dd55851948514cf5a84336d162017c3548d39` | PASS |

## Manual and non-local verification

| Area | Status | Boundary |
| --- | --- | --- |
| Installed native GUI close/restart/reload | MANUAL VERIFICATION REQUIRED | The isolated Rust application round trip passes; an installed interactive desktop process was not exercised in this migration run |
| macOS and Windows package execution | NOT RUN | Local Linux verification cannot establish platform-specific bundle behavior |
| Remote GitHub Actions | NOT RUN | No remote workflow execution was authorized |
| Signing, notarization, and updater delivery | NOT RUN | These release operations are outside local technical migration verification |
| Tag, publication, upload, and release creation | NOT RUN | No release or remote mutation was authorized or performed |

These manual and non-local items do not reduce the local compatibility result because a safe, complete temporary-workspace fixture now covers the required Phase 24 data round trip. They remain relevant to final cross-platform and release acceptance.

## Remaining risks

- The bundled `fpcalc` sidecars are not distribution-ready until the corresponding-source, relinking, and exact FFmpeg/FFT component obligations are reviewed.
- Remote CI and cross-platform bundle behavior cannot be inferred from local YAML validation or a Linux dry run.
- Interactive installed-GUI behavior remains a manual product check even though the isolated application-state restart passes.
- Lifecycle adoption, lifecycle verification, and final migration acceptance have not yet occurred.

## Verification

Local technical and product/data compatibility is `PASS`. The mandatory applicable command matrix passes, the approved Gate B corrections pass, the complete isolated Phase 24 fixture passes, no original product data was opened, and the unexpected product-data change class is empty.

This is the pre-Gate-C state. It does not authorize the technical commit, claim adoption, create lifecycle metadata, approve a final report, or authorize a tag, signing operation, publication, upload, or release.

## Related documents

- [Migration audit](audit.md)
- [Ownership matrix](ownership-matrix.md)
- [Migration plan](migration-plan.md)
- [Template migration overview](template-v1.0.3.md)
- [Persistence definition](../../../def/persistence.md)
- [Track documentation model](../../../def/track-documentation-model.md)
