<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0004: Deterministic document generation

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-15 |
| Executed | 2026-08-13/15 — partial automated execution |
| Requirement | [`REQ-DOC-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-PER-006`](../../def/persistence.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current 2026-08-15 working tree not yet committed; retained package digests remain in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; native disposable document/adoption fixtures |

## Purpose

This plan verifies that versioned templates generate the required factual Markdown and text files deterministically and safely.

## Objective

Accept document generation when identical normalized inputs produce identical bytes, track snapshots contain actual values, conditional facts remain truthful, paths are relative, and unmanaged existing documents are never silently overwritten.

## Scope

### Included

- all eight required generated documents;
- template-version and freshness behavior;
- global-plus-track snapshot inputs;
- deterministic byte output;
- factual language and conditional edits; and
- safe regeneration and collision/adoption behavior.

### Excluded

- SHA-256 list generation;
- certificate rendering; and
- legal review of user-supplied facts.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Mutable global setting is not propagated to an open track | Generated documents keep `Not documented` values | Update the profile, reload/regenerate an open track, and preserve a finalized comparison track |
| Generator invents work or legal claims | Misleading documentation | Use negative branch fixture and prohibited-phrase scan |
| Nondeterministic timestamps or ordering | Integrity changes without domain change | Generate twice from frozen inputs and compare bytes |
| Existing unmanaged file is replaced | User-authored content loss | Seed sentinel document and attempt generation |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A disposable workspace and track are created.
- [ ] Template version under test is identified.
- [ ] Frozen test facts and evidence metadata contain no sensitive data.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Minimal factual track | No external audio, no human editing, human-only artwork, fixed dates and Suno URL on an example domain |
| TD-02 | Fully branched track | External audio, confirmed specific edits, AI-assisted artwork, content-check notes, and synthetic evidence metadata |
| TD-03 | Global profile change | One open and one finalized track; change current workspace artist, profile name, and handle after first generation |
| TD-04 | Unmanaged collision | Sentinel content at `03_DOCUMENTATION/README.md` without managed marker |
| TD-05 | Legacy managed paths | Managed-marker files at `03_DOCUMENTATION/Lyrics.md` and `03_DOCUMENTATION/Styles.md` |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-DOC-001` | Generate documents for TD-01. | All required outputs under `02_SUNO`, `03_DOCUMENTATION`, `04_LICENSES`, and `05_ARTWORK` exist as text files. | The native end-to-end test generated a current set; freshness verification reads and compares all eight required files. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate`; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 2 | `REQ-DOC-001` | Review TD-01 output. | Negative source/editing answers are factual; no arrangement, mixing, mastering, or copyright guarantee is invented. | Not run | NOT RUN | — |
| 3 | `REQ-DOC-001` | Generate TD-02. | Applicable source, human-work, AI, artwork, and disclosure facts appear with evidence references. | Not run | NOT RUN | — |
| 4 | `REQ-DOC-001` | Generate a second time from identical normalized TD-02 input. | Every managed output is byte-identical and ordering is stable. | All eight managed outputs matched committed golden bytes, then a second generation matched the first byte for byte. | PASS | Rust `all_documents_match_golden_bytes_and_exclude_forbidden_content` |
| 5 | `REQ-DOC-001` | Apply TD-03 and regenerate the open track. | Artist, Suno profile, and handle use the updated global values; the finalized track retains its previous snapshot. | The profile and all open-track records were committed together, the open track became stale and regenerated with all three updated values, and the finalized track retained its original profile snapshot. | PASS | Rust `profile_updates_refresh_open_tracks_but_preserve_finalized_snapshots` |
| 6 | `REQ-DOC-001` | Change a confirmed track input. | Affected documents become stale until regeneration; unaffected documents retain their expected state. | Not run | NOT RUN | — |
| 7 | `REQ-PER-006` | Inspect every generated path reference. | Paths are relative to the track root and contain no local absolute path. | Golden outputs were generated from portable evidence paths; private fixture values including an absolute home path were asserted absent from the complete output set. | PASS | Rust `all_documents_match_golden_bytes_and_exclude_forbidden_content` |
| 8 | `REQ-DOC-001` | Attempt generation over TD-04. | Generation stops and requests explicit adoption; sentinel content remains unchanged. | Generation without adoption returned `AdoptionRequired`; the exact binary sentinel remained unchanged and no other managed output was written. | PASS | Rust `adopt_existing_false_leaves_unmanaged_sentinel_unchanged` |
| 9 | `REQ-DOC-001` | Confirm adoption in a fresh fixture. | The sentinel is backed up below `.archive/` before an atomic managed write succeeds. | Confirmed adoption archived the exact sentinel bytes under `.archive/adoptions/` before the managed golden replacement appeared. | PASS | Rust `adopt_existing_true_archives_exact_bytes_before_managed_replacement` |
| 10 | `REQ-DOC-001` | Scan generated prose for prohibited certification or legal-guarantee claims. | No invented legality, ownership, governmental certification, or guaranteed-noninfringement statement exists. | The combined eight-file golden output was scanned against the prohibited-claim list; no forbidden claim was present. | PASS | Rust `all_documents_match_golden_bytes_and_exclude_forbidden_content` |
| 11 | `REQ-DOC-001` | Regenerate with TD-05 present. | Template `1.1` writes `02_SUNO/Lyrics.md` and `02_SUNO/Style.md`, then removes only the old managed files. | Both new Suno documents were generated and both exact-marker legacy files were removed. | PASS | Rust `generation_moves_managed_lyrics_and_style_documents_into_suno_folder` |

## Automated checks

```sh
cd src-tauri
cargo test all_documents_match_golden_bytes_and_exclude_forbidden_content
cargo test adopt_existing
cargo test profile_updates_refresh_open_tracks_but_preserve_finalized_snapshots
cargo test generation_moves_managed_lyrics_and_style_documents_into_suno_folder
```

Expected Rust evidence is `tests::document_generation_is_deterministic_and_requires_adoption`. Attach golden-output or deterministic-comparison results when executed.

## Verification

Evidence includes the template version, normalized fixture, two output-tree digests, snapshot comparison, prohibited-phrase scan, and collision/adoption results.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Negative-only prose review and stale-input propagation remain unexecuted as complete ATP fixtures. | Medium | Product team | Execute steps 2, 3, and 6 with retained output-tree digests. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 4, 5, and 7–11 passed; three mandatory steps remain `NOT RUN`.
- Residual risks: Negative-only prose and stale-input propagation still require complete acceptance fixtures.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Local persistence and recovery](../../def/persistence.md)
- [Legacy managed-document adoption](../../dev/legacy-track-import.md#managed-document-adoption)
