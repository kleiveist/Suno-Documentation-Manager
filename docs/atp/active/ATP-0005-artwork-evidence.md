<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0005: Artwork evidence and content declarations

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-15 |
| Executed | 2026-08-13/15 — partial automated execution |
| Requirement | [`REQ-EVD-001`, `REQ-ART-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current 2026-08-15 working tree not yet committed; retained packaged baseline and digests remain identified in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; temporary native image/evidence fixtures |

## Purpose

This plan verifies native evidence import and replacement, bounded large-file loading, in-app preview, visible type guidance, artwork stage naming, collision protection, and conditional real-person, real-event, and trademark/logo declarations.

## Objective

Accept artwork evidence when real files are copied into contained roles, originals remain unchanged, only actual production stages are required, and positive content declarations request factual notes without producing legal conclusions.

## Scope

### Included

- AI original, AI-edited, human-edited, and final artwork roles;
- native file selection, type validation, safe copy, size, and SHA-256 metadata;
- single-pass streamed copy/hash behavior and bounded routine loading for large project ZIP evidence;
- distinct preview and explicit replacement controls with visible accepted file types;
- image/text preview bounds and metadata-only ZIP preview;
- naming convention and optional-stage behavior;
- destination collision handling; and
- three conditional content checks.

### Excluded

- visible disclosure image processing, covered by ATP-0006;
- legal review of depicted content; and
- remote image services.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Import moves or changes the source | Original evidence loss | Compare source path and hash before and after import |
| Role collision overwrites prior evidence | Evidence loss or false provenance | Import two different files to the same role |
| Every artwork stage is forced | User fabricates nonexistent process | Test minimal AI-only and human-only processes |
| Positive content answer becomes a legal decision | Misleading documentation | Inspect step state and generated factual fields |
| A large project ZIP blocks every later load | Unresponsive application and abandoned import | Verify background single-pass copy/hash and bounded normal inspection |
| A duplicate indexed path reaches SQLite | Raw `UNIQUE(track_id, relative_path)` failure or copied orphan | Reject normal duplicate import before copy and use explicit record replacement |
| Preview reads or expands a large archive | Excess memory and CPU usage | Treat ZIP and unsupported large formats as metadata-only |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A disposable track exists.
- [ ] Test images are valid, synthetic, and contain no real personal data.
- [ ] Expected source hashes and sizes are recorded.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | AI original | Valid PNG with deterministic synthetic pixels |
| TD-02 | Different AI candidate | Valid PNG with same proposed role and filename collision |
| TD-03 | Human-edited and final files | Valid PNG and JPEG synthetic images |
| TD-04 | Invalid image | Text bytes with a `.png` extension |
| TD-05 | Content declarations | Separate `No` and `Yes` answers with neutral synthetic notes |
| TD-06 | Large project archive | Synthetic valid ZIP, including a sparse file above the 64 MiB routine-inspection threshold; retain a 1.3 GB removable-drive fixture for packaged acceptance |
| TD-07 | Preview and replacement | Small valid PNG/text preview fixtures plus two different same-name ZIP/WAV sources |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-EVD-001` | Import TD-01 through the native picker as AI artwork original. | A contained copy is created with role, relative path, validated type, size, and SHA-256 metadata. | Not run | NOT RUN | — |
| 2 | `REQ-EVD-001` | Compare TD-01 before and after import. | The source remains present and byte-identical. | Native import retained the source bytes and created a byte-identical managed copy. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 3 | `REQ-ART-001` | Inspect the proposed managed name. | It follows `<track>_AI_ORIGINAL.png` and does not alter the original source name or file. | The managed path was exactly `05_ARTWORK/My-Track_AI_ORIGINAL.png`. | PASS | Rust `artwork_import_uses_documented_role_naming` |
| 4 | `REQ-EVD-001` | Attempt to import TD-02 to the occupied role/destination. | The application reports a conflict and neither existing destination nor source is overwritten or removed. | Duplicate import returned `Collision`; source and destination bytes remained intact. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 5 | `REQ-EVD-001` | Attempt TD-04 as artwork. | Type validation rejects the file with a controlled error and creates no evidence record. | A `.png` containing plain-text bytes was rejected by signature validation before copy/indexing. | PASS | Rust `artwork_import_uses_documented_role_naming`; [central report](../../dev/acceptance-report.md) |
| 6 | `REQ-ART-001` | Model an AI original followed directly by a final output. | AI-edited and human-edited intermediate stages are not required. | Not run | NOT RUN | — |
| 7 | `REQ-ART-001` | Model the full TD-01/TD-03 production sequence. | Present stages use `AI_ORIGINAL`, optional `AI_EDITED` where supplied, `EDITED`, and `FINAL` roles in order. | Not run | NOT RUN | — |
| 8 | `REQ-ART-001` | Answer `No` to each TD-05 content check and save. | Each branch ends, AI Transparency is deactivated, a regular final Suno JPG/PNG can satisfy the one final-artwork requirement, and the rail refreshes immediately. | Native and frontend evaluation disabled disclosure after three explicit negative answers. Frontend requirements accepted one verified final image without a matching disclosure derivative and marked the completed artwork branch `PASS`. | PASS | Rust `three_negative_content_checks_disable_ai_transparency`, `track_patches_clear_values_from_inactive_conditional_branches`; Vitest `deactivates AI Transparency after three explicit No answers` |
| 9 | `REQ-ART-001` | Answer `Yes` separately for real person, real event, and trademark/logo. | A factual note and configured evidence become applicable; no legal result is generated. | Not run | NOT RUN | — |
| 10 | `REQ-EVD-001` | Remove an imported disposable role through the product action. | Only the explicitly selected managed evidence is affected according to confirmation; the original source remains untouched and workflow reevaluates. | Not run | NOT RUN | — |
| 11 | `REQ-EVD-001` | Inspect every evidence control and use TD-07 after import. | Each control states accepted file types. A present control opens preview, while its separate right-hand upload control asks for explicit replacement. | The typed role mapping covers every evidence role and its ZIP/PNG/WAV examples pass. Complete packaged visual and keyboard inspection remains open. | PARTIAL | Vitest `lists the accepted file types directly for every evidence role`; frontend `inlineEvidenceActions` |
| 12 | `REQ-EVD-001` | Preview TD-07 image and ZIP evidence. | The image appears inside the app. ZIP contents are not expanded or loaded; metadata and a safe explanation appear instead. | Native preview returned a PNG data URL and returned metadata-only state for ZIP with no image or text payload. Packaged popup rendering remains open. | PARTIAL | Rust `evidence_preview_embeds_images_but_does_not_load_zip_archives` |
| 13 | `REQ-EVD-001` | Replace occupied TD-07 evidence through the right-hand upload action. | The existing record is updated without a uniqueness error, the new file becomes active, prior managed bytes are archived, and the source remains unchanged. | Unit and application integration tests retained the evidence ID, activated the new bytes/name, and found the prior bytes below `.archive/evidence-replacements/`. | PASS | Rust `explicit_replacement_archives_previous_bytes_and_reuses_database_identity`; `authoritative_release_and_artwork_roles_are_singular` |
| 14 | `REQ-EVD-001` | Import and repeatedly load TD-06, then run explicit verification. | Copy and SHA-256 share one background stream; normal loading does not repeatedly hash the large file; explicit verification still reads and detects changed bytes. | Copy/hash returned one stream's digest and size. Bounded inspection kept stored verification for unchanged large-file metadata, while explicit SHA-256 verification detected the deliberately mismatched digest. Packaged responsiveness with the retained 1.3 GB fixture remains open. | PARTIAL | Rust `copy_new_hashed_returns_the_digest_from_the_copy_stream`; `large_evidence_load_is_bounded_but_explicit_verification_hashes_it`; native `spawn_blocking` command boundary |
| 15 | `REQ-EVD-001` | Repeat a normal import whose managed relative path is already indexed. | A controlled instruction points to explicit replacement before any copy; no raw SQLite `UNIQUE` error is shown. | Native preflight queries the indexed relative path before copying, and the persistence regression converts a forced relative-path uniqueness conflict to controlled replacement guidance. Complete packaged message execution remains open. | PARTIAL | `WorkspaceApp::import_evidence_from`; Rust `evidence_provenance_fields_round_trip_and_update` |
| 16 | `REQ-ART-001` | Inspect missing requirements with an existing but semantically unsuitable `final_artwork`. | The final artwork appears once under Artwork, states `PNG oder JPG`, and its action safely replaces the existing record instead of attempting a duplicate role import. | Requirement evaluation removed the duplicate Release entry. The missing-evidence renderer resolves the existing role ID and routes its button through explicit replacement. Packaged interaction remains open. | PARTIAL | Vitest `requires a verified generated disclosure artifact for AI artwork`; frontend `renderEvidence` |

## Automated checks

```sh
cd src-tauri
cargo test evidence_import_validates_type_preserves_source_and_rejects_collision
cargo test explicit_replacement_archives_previous_bytes_and_reuses_database_identity
cargo test large_evidence_load_is_bounded_but_explicit_verification_hashes_it
cargo test evidence_preview_embeds_images_but_does_not_load_zip_archives
cargo test three_negative_content_checks_disable_ai_transparency
cd ../frontend
npm test -- --run src/domain/workflow.test.ts
```

Expected Rust evidence includes the collision, replacement/archive, large-file bounded-load, and preview tests named above. Expected Vitest evidence includes the explicit three-`No` Artwork status and evidence-role type-label tests.

## Verification

Evidence includes source and destination digests, relative paths, role metadata, collision output, invalid-type output, and branch screenshots. Do not attach real artwork or personal depictions.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Native picker use, optional/full stage sequences, positive content-declaration branches, product removal, packaged preview/replacement interaction, and a real 1.3 GB removable-drive ZIP remain unexecuted. | Medium | Product team | Execute steps 1, 6, 7, 9–12, and 14–15 with retained UI, responsiveness, memory, and file metadata evidence. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Automated source preservation, naming, type validation, three-`No` workflow completion, safe replacement, bounded large-file inspection, and native preview behavior pass. Packaged UI and real 1.3 GB removable-drive execution remain partial or not run.
- Residual risks: Native picker responsiveness on the target removable filesystem, complete popup/keyboard behavior, positive content declarations, removal, and platform decoder differences are not accepted yet.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-15 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Getting started](../../usr/getting-started.md)
- [ATP-0006: AI disclosure generation](ATP-0006-ai-disclosure-generation.md)
