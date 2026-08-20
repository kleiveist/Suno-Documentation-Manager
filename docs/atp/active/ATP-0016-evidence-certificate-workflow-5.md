<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0016: Evidence and certificate workflow 5.0 release candidate

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-17 |
| Last review | 2026-08-17 |
| Executed | 2026-08-17 — final Rust and frontend suites, frontend production build, 120-dpi visual PDF review, and retained all-facts portable-folder review |
| Requirement | [`REQ-CER-001`, `REQ-CER-002`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-WFL-007` through `REQ-WFL-010`](../../def/workflow-model.md#requirements-and-atp-mapping), [`REQ-PER-009` through `REQ-PER-011`](../../def/persistence.md#requirements-and-atp-mapping) |
| Current target | Product `0.1.0`; workflow `1.7`; document template `1.8`; manifest schema `5`; certificate/PDF format `5.0`; SQLite schema `5` |
| Tested build | Current Product `0.1.0` working tree with the target versions above |
| Environment | Linux x86_64 host toolchain invoked through the Flatpak host boundary; Rust 271 total (270 passed, 0 failed, 1 ignored); Vitest 6 files/113 passed; TypeScript and Vite production build passed (17 modules) |

## Purpose

Verify the release-candidate semantics introduced for the final technical documentation and evidence workflow. This ATP maps user-requested Tests 01–18 to current executable regression coverage without altering the historical results in ATP-0015 or the 2026-08-14 acceptance report.

`PASS` in this ATP means only that the complete expected result of that test was observed in the identified run. It does not mean legal compliance, rights clearance, timestamp-authority trust, or timestamp qualification.

## Scope boundary

The application may technically verify file presence, exact bytes, SHA-256 equality, date-interval coverage, manifest structure, revision association, and deterministic rendering. It must not infer authorship, ownership, non-infringement, license validity, AI-law compliance, timestamp legal qualification, judicial weight, or governmental certification.

`NO`, `N/A`, and `NOT DOCUMENTED` remain distinct. Historical lyrics and `sunoPlanAtCreation` values remain separate legacy data and do not satisfy the new vocal/Suno-field or plan-at-generation facts.

## Acceptance matrix: user Tests 01–18

The automated-evidence column maps each requested scenario to executable regression coverage. The final full-suite results and identified manual reviews below apply to the same final working tree. A row is `PASS` only when its complete expected result was observed.

| Test | Scenario | Expected result | Mapped automated evidence | Actual result | Status |
| --- | --- | --- | --- | --- | --- |
| 01 | `STRUCTURE_ONLY` with `[Intro]`, `[Drop]`, `[Outro]`, explicit `INSTRUMENTAL` intent, and no vocals in the final audio | Workflow passes; generated output prints the canonical classification and intent and presents structure instructions without a vocal-lyrics claim | Rust `vocal_intent_classification_and_final_audio_are_independent`, `structure_only_generation_text_does_not_become_vocal_lyrics_or_override_audio_result`; frontend `TEST 01 accepts an instrumental with bracketed structure instructions and no vocal lyrics` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 02 | Vocal Intent `INSTRUMENTAL` and final audio contains vocals `YES` | Native/UI workflow accepts both independent facts; neither free text, mode, classification, nor audio rewrites Vocal Intent | Rust `vocal_intent_classification_and_final_audio_are_independent`; frontend `keeps Vocal Intent, classification, instrumental mode, and final audio independent` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 03 | Vocal Lyrics plus structure instructions | Exactly one Content Classification value, `MIXED`, is stored and rendered; explicit Vocal Intent and final-audio result remain separate | Rust `vocal_intent_classification_and_final_audio_are_independent`; frontend `requires explicit scalar semantics and accepts MIXED as one classification`, `TEST 03 accepts a vocal track with vocal lyrics` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 04 | Terms PDF with title, provider/source, and retrieval date | Evidence & Licenses requirement passes; summary, manifest/document output, PDF, and evidence register retain the complete metadata and same local Evidence ID | Rust `commercial_terms_require_verified_evidence_with_complete_core_metadata`, `complete_terms_metadata_uses_same_evidence_id_in_summary_and_register`, `renders_complete_terms_record_and_phase_one_timestamp_status`, `global_terms_pdf_import_requires_core_metadata_and_propagates_to_mutable_tracks`; frontend `TEST 04/05 requires complete core metadata for commercial Terms evidence`, `TEST 04/05 identifies complete Terms core metadata without inventing optional facts` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 05 | Commercial track with a Terms file but missing core metadata | No silent PASS; exact incomplete-metadata blocker remains until the facts are supplied | Same positive/negative Rust and frontend Terms fixtures as Test 04, including invalid date handling | Mapped Rust and frontend checks passed in the full suites | PASS |
| 06 | Final generation `2026-08-15`, plan `Premier`, and coverage through that date | PDF records plan at generation and technical coverage `YES` without a license/right conclusion | Rust `renders_plan_at_generation_and_technical_subscription_coverage`, `commercial_generation_must_be_inside_verified_subscription_coverage`; frontend `TEST 06/07 records the plan at generation and evaluates date coverage technically` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 07 | Final generation after the selected coverage end | Technical coverage is `NO` and the commercial workflow blocks | Rust `commercial_generation_must_be_inside_verified_subscription_coverage`; frontend `TEST 06/07 records the plan at generation and evaluates date coverage technically` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 08 | Complete generative-audio questionnaire with disclosure `YES` and negative real-person/event indicators | AI Transparency completes and renders factual indicators/disclosure without an AI-law conclusion | Rust `complete_audio_ai_questionnaire_accepts_explicit_not_documented_indicators`, `each_audio_ai_indicator_requires_an_explicit_tri_state_answer`, `renders_complete_audio_transparency_without_legal_conclusion`; frontend `TEST 08 completes the factual audio AI transparency questionnaire without legal conclusions` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 09 | Commercial generative-AI use with disclosure `NOT DOCUMENTED` | AI Transparency remains visibly incomplete and blocks finalization | Rust `commercial_generative_audio_blocks_not_documented_disclosure`; frontend `TEST 09 keeps commercial generative-AI disclosure NOT DOCUMENTED visibly incomplete` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 10 | Deliberate disclosure `NO` with an optional factual reason | The answer is documented as `NO`; workflow does not treat it as unknown or make a legal judgment | Rust `explicit_no_audio_disclosure_is_complete_without_a_reason`, `renders_conscious_no_disclosure_with_reason`; frontend `TEST 10 accepts a deliberate NO disclosure with an optional factual reason` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 11 | External timestamp evidence references the exact finalized Manifest SHA-256 | Certificate-bound sidecar-v1 and deterministic Markdown/PDF addenda are created through stage → database registration → live publication; claimed/actual/evidence and pinned addendum hashes and provenance are visible; phase-one bytes stay unchanged | Rust `external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision`, `timestamp_publication_recovery_reconciles_pending_stages_without_adopting_orphans`, `verification_pins_published_bytes_and_never_requires_current_renderer_output`, `external_timestamp_addendum_is_complete_factual_and_deterministic`; frontend `TEST 11/12 uses factual external timestamp labels without claiming qualification`, `attaches external timestamp evidence to a finalized track through its dedicated command` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 12 | Timestamp claim references a different SHA-256 | Record remains factual with `Referenced hash match: NO`; no positive integrity or qualification statement is made | Same timestamp Rust/frontend fixtures as Test 11 | Mapped Rust and frontend checks passed in the full suites | PASS |
| 13 | Create revision 2 after timestamping revision 1 | Timestamp sidecars and database records remain bound to revision 1's Certificate ID and archive; revision 2 receives none automatically; archived records remain listed and byte-reverified | Rust `external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision`, `finalized_track_rejects_mutation_until_revision_archives_snapshot` | Mapped Rust checks passed, including archived-sidecar tamper reporting integrity `NO` without changing the base certificate | PASS |
| 14 | Modify a protected evidence file after finalization | Main integrity verification fails and certificate state is invalidated without rewriting the certificate | Rust `hash_verification_detects_changed_deleted_and_added_files`, `external_change_invalidates_certificate_state_without_rewriting_certificate_files`, `externally_deleted_evidence_remains_loadable_and_invalidates_finalized_state` | Mapped Rust checks passed in the full suite | PASS |
| 15 | Release and Suno final export are byte-identical, then one byte changes | Dedicated comparison is `YES` only for equal verified SHA-256 values and changes to `NO`/mismatch after byte change | Rust `byte_identity_is_a_system_verification_over_verified_hashes`, `p0_no_post_editing_derives_production_end_and_identical_release_passes`, `one_changed_audio_byte_is_reported_as_not_identical_in_every_certificate_format`, plus main integrity mutation fixtures | Mapped Rust checks passed in the full suite | PASS |
| 16 | One snapshot contains explicit `NO`, deterministic `N/A`, and `NOT DOCUMENTED` | All three states remain distinct in workflow, presentation, snapshot, and PDF | Rust `preserves_no_na_and_not_documented_as_distinct_states`; frontend `TEST 16 distinguishes NO, NOT DOCUMENTED and deterministically non-applicable answers`, `TEST 16 keeps YES, NO, NOT DOCUMENTED and timestamp verification states distinct` | Mapped Rust and frontend checks passed in the full suites | PASS |
| 17 | PDF contains long URLs, UUIDs, full SHA-256 values, many revisions/subscriptions, Terms, timestamp addendum, and long Suno instructions | Automated extraction retains all values and pagination; manual visual inspection must additionally confirm no clipping, overlap, broken labels, or unreadable wrapping | Rust `combined_long_value_regression_remains_complete_and_paginated`, `long_evidence_paths_wrap_without_truncation`, `large_evidence_register_and_long_timestamp_addendum_paginate_without_truncation`, `external_timestamp_addendum_is_complete_factual_and_deterministic` | Automated extraction, pagination, continuation-header, base/addendum, and footer checks passed. The byte-identical reviewed base PDF was 11 A4 pages, SHA-256 `cac01f5b0bc771e5075e487481c9781efb8239242588557e3835db81ba3fa9e3`; its timestamp addendum was 2 A4 pages, SHA-256 `a64332b2089c7e684ebd3f0325e3bf5f27e21e944ead1ffd69eadc64f523bcb3`. Every page was rendered at 120 dpi and inspected: no clipping, overlap, or broken labels; long URL, UUID, SHA-256, and instruction values wrapped readably; header, Certificate ID, and `Seite X / Y` were present | PASS |
| 18 | Render the same normalized snapshot twice | Same factual content and deterministic document, base-PDF, and addendum-PDF bytes; no UI randomness enters output | Rust `all_documents_are_deterministic_and_exclude_forbidden_content`, `identical_snapshot_produces_identical_pdf_bytes`, `external_timestamp_addendum_is_complete_factual_and_deterministic`, `disclosure_renderer_is_deterministic_and_bottom_right_only`, `end_to_end_documentation_workflow_creates_portable_certificate` | Mapped Rust checks passed, including byte-identical repeated manifest, Markdown, PDF, and hash-set output | PASS |

## Additional semantic-migration audit

| Check | Expected result | Mapped automated evidence | Actual result | Status |
| --- | --- | --- | --- | --- |
| Legacy lyrics and plan JSON | `lyricsSource`/`lyricsText` remain unclassified; `sunoPlanAtCreation` becomes only `legacySunoPlanAtCreation`; new `sunoPlanAtGeneration` remains `NOT DOCUMENTED` until user confirmation | Rust `historical_lyrics_and_plan_keys_remain_unclassified_legacy_values`, `legacy_lyrics_fields_alone_do_not_answer_the_new_documentation_questions`, `legacy_plan_at_creation_does_not_satisfy_plan_at_generation`, `legacy_plan_value_does_not_become_plan_at_generation`, `historical_plan_value_is_not_rendered_as_plan_at_generation` | Mapped Rust checks passed in the full suite | PASS |
| Timestamp sidecar v1 durability and reverification | A custom `Other` anchor must be an unchanged phase-one SHA256SUMS entry; publication is stage → database registration → live; on Unix the new staging/live containers and their parents plus the completed stage/parent are `fsync`ed before registration, and live-parent `fsync` precedes database rollback, while non-Unix directory durability is explicitly best effort; startup completes only registered pending state, removes unregistered staging, and rejects unregistered live orphans; the v1 JSON must equal the canonical immutable bytes, so injected runtime/trust fields are rejected even with a self-renewed hash list; current and archived verification hashes published bytes without re-rendering; archived `revision.json.previous_certificate.certificateId` must match the sidecar and binding/sidecar tampering reports integrity `NO` independently while the unchanged base certificate remains valid | Rust `external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision`, `timestamp_publication_recovery_reconciles_pending_stages_without_adopting_orphans`, `verification_pins_published_bytes_and_never_requires_current_renderer_output`, `legacy_v0_sidecars_remain_self_consistently_verifiable_without_rendering` | Mapped Rust checks and the inspected publication/rollback ordering passed in the final tree | PASS |
| Terms availability invariant | Verified local Terms evidence cannot coexist with `Terms evidence not available: YES`; API mutation is rejected, imported contradiction blocks workflow consistency, and certificate renderers refuse contradictory output | Rust `global_terms_pdf_import_requires_core_metadata_and_propagates_to_mutable_tracks`, `verified_terms_evidence_conflicts_with_an_unavailable_claim`, `rejects_contradictory_terms_availability_statements` | Mapped Rust checks passed in the full suite | PASS |

## Complete release-candidate track review

The requested whole-track scenario combines instrumental bracketed Suno instructions, source code and generated audio, post-processing, Suno IDs/date/model/plan, two subscription records, complete Terms metadata, AI-assisted artwork and human changes, the Audio AI assessment, release MP3/WAV, a byte-identical Suno export, hashes, manifest, at least one revision, and an optional timestamp addendum.

The complete final-tree fixture at `/tmp/.tmpNqK2bz` passed retained artifact review. All 19 phase-one `SHA256SUMS.txt` entries, all 4 `CERTIFICATE_SHA256.txt` entries, and all 4 timestamp-sidecar hash entries verified. Independent reproduction produced byte-identical manifest, Markdown certificate, PDF certificate, and certificate hash set. The all-facts `jq` assertions and decisive managed-document fact checks passed. The archived revision's previous Certificate ID equals the timestamp sidecar's Certificate ID. Its v1 `TIMESTAMP_RECORD.json` pins the Markdown/PDF hashes, records publication-time integrity, and contains no current runtime `integrityVerified` claim.

## Reproducible commands

Run from the repository root:

```sh
npm --prefix frontend test -- --run
npm --prefix frontend run build
cargo test --manifest-path src-tauri/Cargo.toml
python tools/control.py docs index --dry-run
```

Where the local Codex environment lacks Node or Rust, run the frontend and Rust commands through the documented host boundary. Independently verify retained final artifacts with every applicable phase-one, certificate, and timestamp-sidecar SHA-256 list; do not record a PASS without preserving the command output and identifying the build.

## Result

- Overall result: `PASS` for user Tests 01–18 and the complete release-candidate track review on the identified final working tree.
- Automated result: Rust 270 passed, 0 failed, 1 ignored out of 271; Vitest 113 passed, 0 failed across 6 files; the TypeScript/Vite production build passed with 17 modules.
- The ignored Rust check is `security::tests::no_clobber_publish_works_on_configured_removable_filesystem`; it requires an explicitly configured disposable `SUNO_DOC_REMOVABLE_FS_TEST_ROOT` and was not executed in this environment.
- User Tests 01–18 passed their mapped automated and, for Test 17, manual expectations.
- Historical ATP-0015 and acceptance-report results remain unchanged and apply only to their identified earlier builds/formats.
- The complete retained portable-folder review passed with all three applicable hash lists, independent byte reproduction, all-facts assertions, managed-document checks, revision binding, and immutable sidecar-v1 checks.
- No legal assessment or independent external timestamp-authority trust/qualification validation was performed or is claimed; the timestamp checks cover local publication/recovery, registration, hash binding, immutable published bytes, revision association, and current integrity status only.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| RC5-ATP-001 | Resolved: current-format base and timestamp-addendum PDFs received a complete 120-dpi visual inspection. | Medium | Product team | Reviewed all 13 pages with the hashes recorded in Test 17; automated continuation-header and layout assertions also passed. | closed |
| RC5-ATP-002 | Resolved: one retained end-to-end track folder containing the requested facts, evidence, revision, and timestamp addendum was independently reviewed. | High | Product team | Verified `/tmp/.tmpNqK2bz`, all three applicable hash lists, independent reproductions, all-facts/managed-document assertions, revision binding, and sidecar-v1 immutable fields. | closed |
| RC5-ATP-003 | The removable-filesystem no-clobber check was ignored because no explicit disposable filesystem root was configured. This environment-specific check is outside user Tests 01–18. | Low | Product team | Rerun `security::tests::no_clobber_publish_works_on_configured_removable_filesystem` with a dedicated disposable `SUNO_DOC_REMOVABLE_FS_TEST_ROOT` on an applicable target. | open |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Workflow model](../../def/workflow-model.md)
- [Persistence and recovery](../../def/persistence.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
- [Historical ATP-0015](ATP-0015-technical-evidence-certificate.md)
