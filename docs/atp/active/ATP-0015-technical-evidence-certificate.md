<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0015: Technical evidence certificate 4.0

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-16 |
| Last review | 2026-08-17 |
| Executed | 2026-08-16 — certificate 3.0 baseline checks and application-independent artifact review; 2026-08-17 — current Rust test-target compile check, native runtime rerun pending |
| Requirement | [`REQ-CER-001`, `REQ-EVD-002`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-WFL-007`, `REQ-WFL-008`](../../def/workflow-model.md#requirements-and-atp-mapping) |
| Tested build | Product 0.1.0 working tree; workflow 1.4; template 1.6; manifest schema 3; certificate 4.0 |
| Environment | Linux x86_64; Rust test targets compile; native test linking currently lacks WebKit/JavascriptCore system libraries; Vitest/Vite frontend checks and retained certificate 3.0 artifact review |

## Purpose

Verify that a new finalized snapshot is a complete, internally consistent technical production/evidence record without making a legal-certification claim.

## Acceptance steps

| Step | Scenario | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| 1 | Instrumental = Yes, lyrics source = mixed, Human Work includes Lyrics | Native finalization requirement fails until the user corrects the facts; no automatic correction | Native and frontend consistency fixtures cover the contradiction | PASS | Rust `instrumental_contradictions_block_until_explicitly_corrected`; Vitest instrumental contradiction fixture |
| 2 | Clean confirmed instrumental | PDF and Lyrics document contain `Lyrics: N/A – instrumental track` and no invented lyrics | Renderer/template fixtures assert the exact statement | PASS | Rust `renders_clean_instrumental_lyrics_statement_and_scope_boundary`; document rendering tests |
| 3 | Commercial generation inside subscription coverage | PDF reports only factual coverage `YES`, never commercial rights | Native coverage enum and frontend fixture return `YES`; prohibited phrase is absent | PASS | Rust `commercial_generation_must_be_inside_verified_subscription_coverage`; PDF scope test |
| 4 | Generation outside coverage or insufficient dates | Commercial workflow blocks with `NO`/`NOT VERIFIED` semantics | Native coverage fixture covers `NO` and `NOT VERIFIED`; frontend exposes the blocker | PASS | Rust coverage test; Vitest commercial coverage fixture |
| 5 | Select one local Suno Terms PDF in Settings | The native picker opens directly without a metadata form; only a signature-checked PDF is accepted; a hashed `global_copy` is automatically placed in every new/editable project while finalized snapshots remain unchanged | Native and demo adapters use a zero-argument picker, invalid extensions and disguised PDFs are rejected, and existing/new-track propagation plus restart persistence are covered | PASS | Rust `global_terms_pdf_import_requires_no_manual_metadata_and_propagates_to_tracks`; Vitest desktop/demo global-terms importer tests |
| 6 | External timestamp import | PDF records existence and metadata only, without qualification claim | PDF fixture asserts issuer/reference metadata and absence of qualification wording | PASS | Rust `renders_terms_and_external_timestamp_as_factual_local_evidence` |
| 7 | Protected evidence modified after finalization | Existing integrity verification invalidates the certificate presentation | Existing finalized integrity mutation coverage remains active | PASS | Rust finalized evidence/integrity regression tests in `application.rs` |
| 8 | Explicit new revision after change | Old PDF, certificate directory, manifest, and hash list remain archived | Existing revision/archive regression coverage remains active | PASS | Rust `finalized_track_rejects_mutation_until_revision_archives_snapshot` |
| 9 | Identical normalized snapshot rendered twice | Fachlicher PDF content and bytes are deterministic | Same fixture produces identical byte vector | PASS | Rust `identical_snapshot_produces_identical_pdf_bytes` |
| 10 | Open a current finalized folder without the application | Format-4.0 PDF, template-1.6 documents, schema-3 manifest, and both hash files explain the chain independently, including final-Suno fact origin and byte identity | The retained certificate-3.0 commercial folder passed both SHA-256 lists and standalone PDF inspection, but a retained format-4.0 artifact set has not yet been reviewed independently | PARTIAL | Prior headless finalized artifact plus `sha256sum -c` and standalone PDF text extraction; current artifact rerun pending |
| 11 | Import a WAV with one valid structured `made with suno studio` record | The exact structured value, valid `created` timestamp/date, `id`, technical audio facts, evidence ID, and SHA-256 are persisted as evidence-derived metadata; only an empty final-generation date is filled, while download and final-export dates remain untouched | Focused parser/application fixtures cover the exact metadata and date boundary and compile in the current Rust test target; native runtime execution is pending in this environment | PARTIAL | Rust `reads_pcm_properties_and_suno_info_comment`, `p0_suno_import_persists_exact_metadata_and_only_derives_generation_date` |
| 12 | Compare the no-post-editing and post-editing branches | Explicit `No` may derive production end from the valid generation date; `Yes` never leaves an automatic production end and requires a real user-confirmed end when applicable | Focused application fixtures cover both controller orders, manual override, subscription coverage, and the no-download-inference rule; native runtime execution is pending | PARTIAL | Rust `p0_no_post_editing_derives_production_end_and_identical_release_passes`, `p0_post_editing_never_leaves_an_automatic_production_end`, `p0_metadata_derived_generation_date_feeds_subscription_coverage` |
| 13 | Import ordinary, malformed, unsafe, or ambiguous WAV metadata and introduce a manual date conflict | The WAV remains importable where structurally valid, no unsupported fact is invented, and a contradiction or ambiguous Suno record blocks through the existing workflow step rather than a parallel validator | Parser, workflow, and application fixtures cover fallback, bounds, ambiguity, exact raw consistency, and dynamic blocking issues; native runtime execution is pending | PARTIAL | Rust `ordinary_wav_is_successful_without_invented_suno_metadata`, `distinct_suno_records_are_preserved_but_not_arbitrarily_selected`, `metadata_conflict_is_dynamic_and_blocks_the_existing_suno_step`, `p0_manual_generation_conflict_is_preserved_and_blocks_the_workflow` |
| 14 | Import byte-identical release/export evidence, then replace or remove the Suno export | System verification reports the generic equal-hash pair and dedicated release/export identity; replacement/removal updates only values still owned by that evidence origin and preserves manual/download/final-export facts | Current workflow/application fixtures cover deterministic pairs and origin-aware reconciliation and compile in the Rust test target; native runtime execution is pending | PARTIAL | Rust `byte_identity_is_a_system_verification_over_verified_hashes`, `p0_replacing_current_suno_export_updates_only_system_owned_values`, `p0_removing_suno_export_clears_only_automatic_values` |
| 15 | Open a finalized pre-metadata snapshot, then explicitly create a revision | Loading/listing/reopening leaves database rows and certificate bytes unchanged; only the new mutable revision analyzes the carried WAV | The focused fixture asserts exact JSON rows, timestamps, facts, origins, and certificate bytes before/after load and then checks revision analysis; native runtime execution is pending | PARTIAL | Rust `p0_finalized_pre_metadata_record_is_not_backfilled_on_load` |
| 16 | Inspect current manifest and human-readable certificate renderers | Manifest schema 3 contains fact origins, Suno summary, byte-identical pairs, consistency issues, unambiguous automatic relationships, and full evidence metadata; certificate/PDF format 4.0 presents a compact summary without raw technical ID/timestamp fields | Manifest-relationship and compact-PDF fixtures compile and assert current format handling; native runtime execution and retained-artifact review are pending | PARTIAL | Rust `automatic_relationships_cover_only_unambiguous_adjacent_role_pairs`, `explicit_lineage_disambiguates_multiple_sources_without_cartesian_products`, `renders_compact_suno_automation_without_raw_timestamp_or_technical_id`, `current_and_previous_pdf_certificate_formats_remain_recognizable` |

## Automated checks

```sh
npm --prefix frontend test -- --run
npm --prefix frontend run build
cargo check --manifest-path src-tauri/Cargo.toml --tests
cargo test --manifest-path src-tauri/Cargo.toml   # or the documented headless core harness where desktop WebKit libraries are unavailable
cargo test --manifest-path src-tauri/Cargo.toml p0_
cargo test --manifest-path src-tauri/Cargo.toml renders_compact_suno_automation_without_raw_timestamp_or_technical_id
python tools/control.py docs index --dry-run
```

## Result

- Overall result: `PARTIAL`
- Retained certificate-3.0 baseline: steps 1–9 pass; the prior artifact portion of step 10 passed.
- Current certificate-4.0 implementation: focused Rust test targets for steps 11–16 compile, but native runtime execution and a retained application-independent 4.0 artifact review remain pending.
- Residual risk: current behavior is not promoted to acceptance `PASS` solely from successful compilation or the older 3.0 artifact.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Current native Rust test execution and standalone certificate-4.0 artifact inspection are incomplete because the review host lacks the desktop WebKit/JavascriptCore linker libraries. | High | Product team | Rerun the focused and full native suites in a provisioned environment, retain a finalized 4.0 fixture, verify both hash lists, and inspect manifest/PDF content independently. | open |

## Scope boundary

No step accepts wording that confirms authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, governmental certification, or legal qualification of a timestamp.

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Workflow model](../../def/workflow-model.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
