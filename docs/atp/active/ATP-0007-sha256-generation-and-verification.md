<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0007: SHA-256 generation and verification

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-15 |
| Executed | 2026-08-13/15 — partial automated execution |
| Requirement | [`REQ-HSH-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571`; package digests in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; native disposable track trees |

## Purpose

This plan verifies native SHA-256 set construction, compatible relative-path output, immediate reread verification, exclusions, and mismatch detection.

## Objective

Accept integrity handling when every required current track file is listed once, every excluded area stays absent, native verification counts match, and a changed file produces a blocking failure.

## Scope

### Included

- release, Suno, documentation, license, and artwork files;
- documented exclusion rules;
- deterministic root-relative list format;
- native generation and verification; and
- independent compatibility check where available.

### Excluded

- certificate artifact hashes, covered by ATP-0009;
- digital signatures; and
- proof of authorship or legal compliance.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| A required file is omitted | Undetected modification | Build an explicit expected-set fixture |
| Hash file or archive hashes itself | Unstable recursive integrity set | Assert all exclusions exactly |
| Verification trusts generated in-memory bytes | Disk corruption goes unnoticed | Modify disk file and require reread mismatch |
| Absolute paths leak | Non-portable manifest and privacy issue | Parse every list path and move the track fixture |
| A large-file pass appears frozen | Duplicate actions, interrupted verification, or premature drive removal | Stream real byte/file/current-path progress from native bounded reads and distinguish the calculation and reread phases |
| Hashing blocks the desktop event loop | Progress card and animations cannot repaint | Dispatch calculation and verification through Tauri's blocking runtime and retain the scoped progress channel |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A disposable track contains one small file in every included category.
- [ ] Excluded directories and certificate placeholders contain sentinel files.
- [ ] The expected root-relative file set and independent SHA-256 values are recorded.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Included set | Synthetic files in `01_RELEASE`, `02_SUNO`, `03_DOCUMENTATION`, `04_LICENSES`, and `05_ARTWORK` |
| TD-02 | Excluded set | Sentinels in `.archive`, `.summary`, `06_CERTIFICATE`, and workspace `.suno-doc` plus the hash file itself |
| TD-03 | Changed file | A copy of one TD-01 file modified after list generation |
| TD-04 | Moved track | Byte-identical track copied to a different local parent path |
| TD-05 | Large file | Synthetic regular file larger than the native progress-report interval |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-HSH-001` | Generate hashes for TD-01 and TD-02. | `03_DOCUMENTATION/SHA256SUMS.txt` is created by native code. | Native Rust generation created the main hash list in a disposable track. | PASS | Rust `hash_generation_verifies_exact_set_and_detects_added_file`; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 2 | `REQ-HSH-001` | Compare listed paths with the expected TD-01 set. | Every included regular file appears exactly once and the count matches. | The test asserted an exact two-file current set and a matching count; parser code rejects duplicate paths. | PASS | Rust integrity and certificate parser tests |
| 3 | `REQ-HSH-001` | Search for every TD-02 sentinel and excluded prefix. | No excluded file, hash list self-entry, certificate, archive, summary, or workspace management data appears. | Not run | NOT RUN | — |
| 4 | `REQ-HSH-001` | Inspect path format. | Every path is normalized and relative to the track root; no local absolute path exists. | Native entries were emitted from stripped track-root paths and accepted by the strict relative-path parser. | PASS | Rust `hash_generation_verifies_exact_set_and_detects_added_file`; static containment review |
| 5 | `REQ-HSH-001` | Complete native verification immediately after generation. | Generated and verified counts match, all entries pass, and the integrity step reports `PASS`. | Immediate reread verification reported two generated/two verified entries and `verified = true`. | PASS | Rust `hash_generation_verifies_exact_set_and_detects_added_file` |
| 6 | `REQ-HSH-001` | Run the optional independent check from the track root. | `sha256sum -c 03_DOCUMENTATION/SHA256SUMS.txt` accepts the format and reports each test file valid. | Not run | NOT RUN | — |
| 7 | `REQ-HSH-001` | Apply TD-03 and verify again without regenerating. | The changed path is identified, result is `FAIL`, and finalization becomes blocked. | Not run | NOT RUN | — |
| 8 | `REQ-HSH-001` | Remove one listed disposable file and verify again. | The missing path is identified and the result blocks finalization. | Not run | NOT RUN | — |
| 9 | `REQ-HSH-001` | Regenerate after intentionally accepting the new working revision. | The new set is written atomically, reread, and verified; partial output is not exposed. | Hash generation published a newly calculated exact set through the atomic-write helper, immediately reread and verified it, while atomic-write failure fixtures preserved the prior destination and cleaned temporary state. | PASS | Rust `hash_generation_verifies_exact_set_and_detects_added_file`; `atomic_writes_publish_complete_bytes_and_never_clobber_new_files`; `atomic_and_copy_failures_preserve_existing_state_and_clean_temporaries` |
| 10 | `REQ-HSH-001` | Verify TD-04 from its new root. | Root-relative entries remain valid without editing the list. | Not run | NOT RUN | — |
| 11 | `REQ-HSH-001` | Calculate and verify TD-05 while collecting native progress. | Intermediate events contain actual processed bytes below the total, completed-file counts, a root-relative current path, and distinct calculation and verification phases. The frontend derives a monotonic phase-aware display without treating progress as the integrity result. | A file larger than the 8 MiB report interval produced intermediate byte events, file completion, list-write, reread-verification, and comparison stages. The scoped IPC adapter and frontend phase mapping passed their tests. | PASS | Rust `hashing_progress_reports_real_bytes_files_and_verification_stages`; frontend `streams native integrity progress through a scoped IPC channel`, `maps real native work counters into honest operation progress` |

## Automated checks

```sh
cd src-tauri
cargo test hash_generation_verifies_exact_set_and_detects_added_file
cargo test hash_verification_detects_changed_deleted_and_added_files
cargo test hashing_progress_reports_real_bytes_files_and_verification_stages
```

Expected Rust evidence is `tests::hash_generation_and_verification_detect_changes`.

Optional independent verification runs from the disposable track root:

```sh
sha256sum -c 03_DOCUMENTATION/SHA256SUMS.txt
```

## Verification

Attach the expected/actual set comparison, native counts, mismatch output, moved-track result, and automated report. Hashes alone are not a claim about provenance or legality.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | The full exclusion sentinel matrix, independent CLI check, compound changed/missing-file ATP steps, and copied-root verification remain unexecuted. | Medium | Product team | Execute steps 3, 6–8, and 10 with retained manifests and command output. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 2, 4, 5, 9, and 11 passed; five mandatory steps remain `NOT RUN`.
- Residual risks: Independent verification, compound mismatch reporting, and portability remain unaccepted through the specified fixtures.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track integrity model](../../def/track-documentation-model.md#integrity-set)
- [Finalizing a track](../../usr/finalizing-a-track.md#generate-and-verify-sha-256)
- [ATP-0008: Finalization gate](ATP-0008-finalization-gate.md)
