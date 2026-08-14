<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0009: Completion certificate and evidence manifest generation

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-14 |
| Executed | 2026-08-13/14 — partial automated execution |
| Requirement | [`REQ-CER-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-PER-005`, `REQ-PER-006`](../../def/persistence.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; regression automation rerun at implementation commit `b7e9797b277f0bcac58d4503049002e354cb93fb`; previously retained package digests still identify the older stabilization build in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; native ready-track, artifact cross-check, and failure-injection fixtures |

## Purpose

This plan verifies factual certificate content, portable JSON manifest structure, certificate-integrity hashes, and the mandatory limitation disclaimer.

## Objective

Accept certificate generation when a ready track produces a complete self-contained artifact set with relative paths, exact hashes, workflow/app versions, no blocking deviation, and no claim of legal or governmental certification.

## Scope

### Included

- `DOCUMENTATION_CERTIFICATE.md` required fields and disclaimer;
- `EVIDENCE_MANIFEST.json` schema and relative references;
- selected global subscription evidence copied into the track with exact materialized coverage dates;
- `CERTIFICATE_SHA256.txt` contents and verification; and
- atomic all-or-nothing finalized presentation.

### Excluded

- qualified signatures, identity proof, or timestamp authority;
- certificate invalidation, covered by ATP-0010; and
- remote publication.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Manifest leaks an absolute local path | Privacy and portability failure | Recursively inspect every string path value |
| Certificate overstates meaning | Misleading legal reliance | Match mandatory disclaimer and prohibited-claim scan |
| Hash list omits one certificate input | Undetected artifact change | Compare exact required set and independently verify |
| Partial write still marks finalized | False snapshot | Inject a write failure before index commit |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment, build, application version, and workflow version are identified.
- [ ] ATP-0008-ready synthetic data is available without claiming ATP execution.
- [ ] Selected reusable subscription evidence is synthetic and covers the fixture dates.
- [ ] Main `SHA256SUMS.txt` has a passing native verification result.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Ready track | Artist `Acceptance Artist`, track `Acceptance Track`, workflow `suno-track` `1.0`, fixed evidence roles |
| TD-02 | N/A details | At least one legitimate conditional N/A with a non-empty reason |
| TD-03 | Global subscription evidence | Synthetic PDF registered globally and selected for the production period |
| TD-04 | Failure injection | Native test hook that fails one certificate artifact write before final state commit |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-PER-005` | Prepare TD-01 with TD-03 selected and finalize. | A contained copy of TD-03 exists in the track, is hashed, and its relative path appears as evidence. | A covering synthetic PDF was registered globally, copied below `04_LICENSES`, byte/hash/provenance checked, included in the main hash set, and represented in the finalized manifest. | PASS | Rust end-to-end and `global_subscription_evidence_requires_pdf_signature_and_covering_dates` tests |
| 2 | `REQ-CER-001` | Inspect `06_CERTIFICATE/`. | It contains exactly the managed certificate Markdown, JSON manifest, and certificate hash list for this revision. | The staged set contained exactly the three managed files and was atomically published; the end-to-end test asserted all three destinations. | PASS | Rust end-to-end and strict certificate-set parser tests; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 3 | `REQ-CER-001` | Review certificate fields. | Certificate ID, track, artist, workflow ID/version, app version, timestamp, mandatory result, TD-02 reasons, evidence count, selected hashes, blocking deviations, and `DOCUMENTATION COMPLETE` are present. | Not run | NOT RUN | — |
| 4 | `REQ-CER-001` | Review the certificate ending and scan for prohibited claims. | The mandatory disclaimer meaning is present and no governmental, legal-advice, authorship, or compliance determination is asserted. | Static review confirmed the mandatory limitation wording and found no affirmative prohibited claim. | PASS | [Central execution report](../../dev/acceptance-report.md); `src-tauri/src/certificate.rs` |
| 5 | `REQ-CER-001` | Parse `EVIDENCE_MANIFEST.json`. | Valid JSON contains `schema_version: 1` and track, artist, workflow, finalization, steps, evidence, hashes, and certificate objects; global evidence exposes its source record ID plus concrete `coverageStart` and `coverageEnd`. | Not run | NOT RUN | — |
| 6 | `REQ-PER-006` | Inspect all manifest path values. | Every file path is normalized and relative to the track root; no drive, home, workspace absolute path, or traversal exists. | The integrated manifest contained portable evidence paths, did not contain the workspace root, and passed strict relative-path checks. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate` and hash-parser tests |
| 7 | `REQ-PER-005` | Compare manifest evidence records to the selected global record and current portable files. | Counts, sizes, roles, relative paths, SHA-256 values, source global-evidence ID, and exact materialized start/end dates match; no cadence extrapolation is required. | Not run | NOT RUN | — |
| 8 | `REQ-CER-001` | Inspect `CERTIFICATE_SHA256.txt`. | It hashes the main hash list, manifest, and certificate Markdown; it has no self-entry. | The exact-set parser accepted only the three required inputs and rejected incomplete, duplicate, extra, and self-entry sets. | PASS | Rust `certificate_hash_parser_requires_exact_complete_unique_set` |
| 9 | `REQ-CER-001` | Verify certificate hashes natively and, where available, with `sha256sum`. | Every entry passes and the finalized state is committed only after success. | Not run | NOT RUN | — |
| 10 | `REQ-CER-001` | Repeat with TD-04. | No partial artifact set is presented as finalized and a controlled error leaves a recoverable working state. | Six deterministic publication failpoints covered staging creation, each artifact write, publish rename, and post-publish verification; each returned a controlled error, preserved any prior set, removed staging, and exposed no partial finalized set. A separate DB-commit failure rolled publication back and reopened cleanly. | PASS | Rust certificate publication failure tests; `finalization_database_commit_failure_rolls_back_publication_and_reopens_cleanly` |

## Automated checks

```sh
cd src-tauri
cargo test finalized_certificate_fields_cross_check_sqlite_track_evidence_hashes_and_manifest
cargo test publication_failure
cargo test finalization_database_commit_failure_rolls_back_publication_and_reopens_cleanly
```

Expected Rust evidence is `end_to_end_documentation_workflow_creates_portable_certificate`, `global_subscription_evidence_requires_pdf_signature_and_covering_dates`, and `finalized_certificate_fields_cross_check_sqlite_track_evidence_hashes_and_manifest`.

Optional independent verification from the track root:

```sh
sha256sum -c 06_CERTIFICATE/CERTIFICATE_SHA256.txt
```

## Verification

Attach redacted synthetic certificate/manifest artifacts, JSON schema checks, exact hash-set comparison, prohibited-claim scan, failure-injection result, and automated report.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | The compound field-by-field/JSON/data review and independent CLI verification remain unexecuted as ATP steps, although native cross-check fixtures cover their core data relationships. | High | Product team | Execute steps 3, 5, 7, and 9 with retained certificate artifacts and independent `sha256sum`. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 2, 4, 6, 8, and 10 passed; four mandatory steps remain `NOT RUN`.
- Residual risks: Independent CLI verification and retained field-by-field artifact review remain open.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation certificate model](../../def/track-documentation-model.md#certificate-artifacts)
- [Finalizing a track](../../usr/finalizing-a-track.md)
- [ATP-0010: Certificate invalidation](ATP-0010-certificate-invalidation-and-revision.md)
