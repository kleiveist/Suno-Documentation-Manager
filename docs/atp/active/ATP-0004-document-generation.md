<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0004: Deterministic document generation

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-13 |
| Executed | 2026-08-13 — partial automated execution |
| Requirement | [`REQ-DOC-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-PER-006`](../../def/persistence.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; unversioned source tree; Linux package digests in the central report |
| Environment | Linux `7.0.8-1-cachyos` `x86_64`; native temporary end-to-end fixture |

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
| Mutable global setting changes old output | Historical snapshot becomes inaccurate | Change defaults after first generation and compare |
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
| TD-03 | Global profile change | Same track snapshot; change current workspace artist and plan after first generation |
| TD-04 | Unmanaged collision | Sentinel content at `03_DOCUMENTATION/README.md` without managed marker |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-DOC-001` | Generate documents for TD-01. | All required outputs under `02_SUNO`, `03_DOCUMENTATION`, `04_LICENSES`, and `05_ARTWORK` exist as text files. | The native end-to-end test generated a current set; freshness verification reads and compares all eight required files. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate`; [final suite](../../../.report/test-report-20260813-144834-suite-all-ok.md) |
| 2 | `REQ-DOC-001` | Review TD-01 output. | Negative source/editing answers are factual; no arrangement, mixing, mastering, or copyright guarantee is invented. | Not run | NOT RUN | — |
| 3 | `REQ-DOC-001` | Generate TD-02. | Applicable source, human-work, AI, artwork, and disclosure facts appear with evidence references. | Not run | NOT RUN | — |
| 4 | `REQ-DOC-001` | Generate a second time from identical normalized TD-02 input. | Every managed output is byte-identical and ordering is stable. | Not run | NOT RUN | — |
| 5 | `REQ-DOC-001` | Apply TD-03 and regenerate without changing the track snapshot. | The track's snapshotted artist and plan remain unchanged in output. | Not run | NOT RUN | — |
| 6 | `REQ-DOC-001` | Change a confirmed track input. | Affected documents become stale until regeneration; unaffected documents retain their expected state. | Not run | NOT RUN | — |
| 7 | `REQ-PER-006` | Inspect every generated path reference. | Paths are relative to the track root and contain no local absolute path. | Not run | NOT RUN | — |
| 8 | `REQ-DOC-001` | Attempt generation over TD-04. | Generation stops and requests explicit adoption; sentinel content remains unchanged. | Not run | NOT RUN | — |
| 9 | `REQ-DOC-001` | Confirm adoption in a fresh fixture. | The sentinel is backed up below `.archive/` before an atomic managed write succeeds. | Not run | NOT RUN | — |
| 10 | `REQ-DOC-001` | Scan generated prose for prohibited certification or legal-guarantee claims. | No invented legality, ownership, governmental certification, or guaranteed-noninfringement statement exists. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test document_generation_is_deterministic_and_requires_adoption
```

Expected Rust evidence is `tests::document_generation_is_deterministic_and_requires_adoption`. Attach golden-output or deterministic-comparison results when executed.

## Verification

Evidence includes the template version, normalized fixture, two output-tree digests, snapshot comparison, prohibited-phrase scan, and collision/adoption results.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Golden prose, repeated byte comparison, snapshot/staleness changes, complete adoption, and prohibited-claim fixture scans were not executed. | Medium | Product team | Execute steps 2–10 with retained output-tree digests and adoption backup evidence. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Step 1 passed through the native integrated path; nine mandatory steps remain `NOT RUN`.
- Residual risks: Determinism and archive-before-adoption behavior have implementation support but no complete acceptance fixture.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Local persistence and recovery](../../def/persistence.md)
- [Legacy managed-document adoption](../../dev/legacy-track-import.md#managed-document-adoption)
