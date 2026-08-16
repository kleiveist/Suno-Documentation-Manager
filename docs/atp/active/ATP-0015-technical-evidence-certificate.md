<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0015: Technical evidence certificate 3.0

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-16 |
| Last review | 2026-08-16 |
| Executed | 2026-08-16 — automated implementation checks and application-independent artifact review |
| Requirement | `REQ-CER-001`, `REQ-WFL-007` |
| Tested build | Product 0.1.0 working tree; workflow 1.3; template 1.5; certificate 3.0 |
| Environment | Linux x86_64; Rust compile/headless core fixtures and Vitest/Vite frontend checks |

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
| 10 | Open finalized folder without the application | PDF, README, AI_USAGE, Suno/license documents, manifest, and hash files explain the chain independently | A synthetic commercial finalized folder was inspected directly from the filesystem: both SHA-256 lists passed, all expected files were present, and extracted PDF text exposed A–J sections, exact instrumental status, coverage, terms PDF/timestamp evidence, origin labels, full hashes, page IDs, and scope limits | PASS | Headless finalized artifact plus `sha256sum -c` and standalone PDF text extraction |

## Automated checks

```sh
npm --prefix frontend test -- --run
npm --prefix frontend run build
cargo check --manifest-path src-tauri/Cargo.toml --tests
cargo test --manifest-path src-tauri/Cargo.toml   # or the documented headless core harness where desktop WebKit libraries are unavailable
python tools/control.py docs index --dry-run
```

## Result

- Overall result: `PASS`
- Automated implementation coverage: steps 1–9 pass.
- Application-independent retained-artifact inspection: step 10 passes.

## Scope boundary

No step accepts wording that confirms authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, governmental certification, or legal qualification of a timestamp.

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Workflow model](../../def/workflow-model.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
