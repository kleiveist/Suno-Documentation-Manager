<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0006: Local AI artwork disclosure generation

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-15 |
| Executed | 2026-08-13/15 — partial automated execution |
| Requirement | [`REQ-ART-002`](../../def/track-documentation-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current 2026-08-15 working tree not yet committed; retained packaged baseline and digests remain identified in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; local Rust image fixture; packaged Debian app was launched offline, but the complete disclosure GUI path was not finished |

## Purpose

This plan verifies the project transparency policy and reproducible local generation of a visible AI artwork disclosure.

## Objective

Accept disclosure generation when applicable artwork produces a separate visible output with configurable text and placement, preserves the original exactly, records the process, and requires no remote image service.

## Scope

### Included

- the three transparency-policy modes;
- AI-generated and AI-assisted applicability;
- local text rendering and placement;
- original/output traceability and deterministic behavior; and
- `AI_USAGE.md` and `artwork_process.md` facts.

### Excluded

- claims about legal watermark requirements;
- remote AI image generation; and
- general-purpose image editing.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Original is overwritten | Loss of primary evidence | Record digest and verify a separate output path |
| Disclosure is invisible or clipped | Transparency policy not satisfied | Inspect pixels at configured position and visual evidence |
| Rendering depends on network/font service | Offline failure or nondeterminism | Disable network and bundle deterministic rendering assets |
| Policy wording asserts a legal mandate | Misleading user guidance | Review UI and generated documents for approved terminology |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A disposable AI-assisted track has TD-01 imported as `AI_ORIGINAL`.
- [ ] Expected input digest and dimensions are recorded.
- [ ] Test data contains no real person, trademark, or private information.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | AI original | Fixed synthetic 1024×1024 PNG |
| TD-02 | Default disclosure | Policy `Always add visible AI disclosure`, text `AI-assisted`, bottom-right placement |
| TD-03 | Custom disclosure | Short visible text at the sole supported fixed bottom-right placement |
| TD-04 | Human-only artwork | Same dimensions, origin declared human-only |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-ART-002` | Open global policy defaults. | `Always add visible AI disclosure` is selected and described as project transparency policy, not universal law. | Not run | NOT RUN | — |
| 2 | `REQ-ART-002` | Generate TD-02 with runtime network disabled. | Processing completes locally and creates a separate final candidate. | Not run | NOT RUN | — |
| 3 | `REQ-ART-002` | Compare TD-01 before and after, then request the identical disclosure again. | Path, size, and SHA-256 are unchanged; no write targets the original; an identical repeated request succeeds idempotently without another file or evidence row. | The local generator retained the original SHA-256 and created one distinct `AI_EDITED` output. The end-to-end integration repeated the same source/version/text request successfully, reported the existing output, and retained exactly one edited evidence item. | PASS | Rust `artwork_disclosure_preserves_original_and_creates_traceable_copy`; `end_to_end_documentation_workflow_creates_portable_certificate` |
| 4 | `REQ-ART-002` | Inspect the TD-02 output. | `AI-assisted` is visible, not clipped, and located at the configured bottom-right position. | Not run | NOT RUN | — |
| 5 | `REQ-ART-002` | Repeat TD-02 from identical input and settings. | Output bytes or the documented deterministic pixel representation match according to the implementation contract. | Two renders from identical 640×640 pixels and custom text produced identical raw output; changed pixels were asserted to remain in the fixed bottom-right region. | PASS | Rust `disclosure_renderer_is_deterministic_and_bottom_right_only` |
| 6 | `REQ-ART-002` | Generate TD-03. | The supported custom text appears at the fixed bottom-right placement and lineage metadata records the exact normalized text. | Not run | NOT RUN | — |
| 7 | `REQ-ART-002` | Review `AI_USAGE.md` and `artwork_process.md`. | Both identify service, AI base image, human changes, policy, applied result, text, and final relative output. | Not run | NOT RUN | — |
| 8 | `REQ-ART-002` | Set policy to `Decide per artwork`. | The track requires an explicit decision; no automatic claim or silent processing occurs. | Not run | NOT RUN | — |
| 9 | `REQ-ART-002` | Set policy to `No automatic visible disclosure`. | The choice and result are documented; the app does not falsely report that disclosure was applied. | Not run | NOT RUN | — |
| 10 | `REQ-ART-002` | Use TD-04. | AI disclosure requirements are excluded; the AI Transparency step accepts N/A only with a saved reason, and the human-only original is not processed automatically. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test artwork_disclosure_preserves_original_and_creates_traceable_copy
cargo test disclosure_renderer_is_deterministic_and_bottom_right_only
cargo test end_to_end_documentation_workflow_creates_portable_certificate
```

Expected Rust evidence is `tests::artwork_disclosure_preserves_original`. Attach pixel/digest comparison, offline processing, branch, and document-output results when executed.

## Verification

The reviewer checks separate input/output paths, input digest preservation, visible output, reproducibility evidence, exact policy terminology, and factual generated documentation.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Complete GUI-path network isolation, visual clipping review, custom-output metadata review, policy branches, and process-document review remain unexecuted. The version 0.1 contract intentionally supports bottom-right placement only. | High | Product team | Execute steps 1, 2, 4, and 6–10 against the fixed bottom-right contract. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Original preservation and deterministic fixed placement (steps 3 and 5) passed; eight mandatory acceptance steps remain `NOT RUN`.
- Residual risks: Visibility, placement, offline operation, policy enforcement, and codec consistency are not accepted yet.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-15 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md#artwork-stages-and-naming)
- [Finalizing a track](../../usr/finalizing-a-track.md#review-artwork-transparency)
- [ATP-0005: Artwork evidence](ATP-0005-artwork-evidence.md)
