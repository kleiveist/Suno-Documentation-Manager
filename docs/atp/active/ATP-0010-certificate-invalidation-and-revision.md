<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0010: Certificate invalidation and revision

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-14 |
| Executed | 2026-08-13/14 — partial automated execution |
| Requirement | [`REQ-WFL-005`, `REQ-WFL-006`](../../def/workflow-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571`; package digests in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; finalized native revision and workflow-upgrade fixtures |

## Purpose

This plan verifies that post-finalization changes invalidate the matching claim, preserve the old snapshot, and require a new archived revision before another finalization.

## Objective

Accept revision handling when a protected byte change is detected, the current certificate is shown invalid, no finalized artifact is silently replaced, and an explicit revision archives the prior certificate state with its original workflow version.

## Scope

### Included

- protected file modification, deletion, and addition behavior;
- invalid-certificate presentation and blocker;
- explicit revision creation and archival structure;
- successful refinalization after full reevaluation; and
- old versus current workflow-version handling.

### Excluded

- editing archive contents;
- remote version control; and
- cryptographic signatures.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Changed final track still appears valid | False integrity claim | Modify one listed byte and reopen/verify |
| Refinalization overwrites old artifacts | Loss of audit trail | Digest certificate files before revision and compare archive |
| Workflow upgrade rewrites old meaning | Historical result corruption | Finalize under 1.0, load a newer definition, and inspect old archive |
| Archive enters current hash list | Recursive or unstable integrity | Inspect inclusion/exclusion sets after revision |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A synthetic track has a verified finalized certificate fixture.
- [ ] Original certificate, manifest, hash-list, and protected-file digests are recorded.
- [ ] A test-only newer workflow definition can be selected without modifying the archived fixture.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Finalized revision | Valid workflow `suno-track` `1.0` certificate set |
| TD-02 | Modified file | One-byte change to a protected documentation file |
| TD-03 | Missing file | Remove one disposable protected evidence file in a fixture copy |
| TD-04 | Workflow update | Recognized test workflow version newer than `1.0` |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-WFL-006` | Open and verify untouched TD-01. | It remains `FINALIZED`, all integrity checks pass, and no revision action is forced. | Not run | NOT RUN | — |
| 2 | `REQ-WFL-006` | Apply TD-02 and verify. | The exact mismatch is reported, certificate presentation becomes invalid, and finalization validity is withdrawn. | A protected release file was changed externally; reload returned a failed integrity set with path-level mismatch state and an invalidated certificate. | PASS | Rust `external_change_invalidates_certificate_state_without_rewriting_certificate_files`; integrity exact-set test |
| 3 | `REQ-WFL-006` | Inspect certificate artifacts immediately after mismatch. | Original certificate, manifest, and certificate hash files remain byte-identical; none is silently regenerated. | Byte snapshots of all three certificate files were identical before and after invalidation. | PASS | Rust `external_change_invalidates_certificate_state_without_rewriting_certificate_files` |
| 4 | `REQ-WFL-006` | Repeat with TD-03 in a fresh copy. | The missing path is reported and the certificate is invalid. | Deleting indexed release evidence kept its relative record loadable with `Evidence file is missing` and invalidated the certificate. | PASS | Rust `externally_deleted_evidence_remains_loadable_and_invalidates_finalized_state` |
| 5 | `REQ-WFL-006` | Choose `Create new revision` after TD-02. | A unique `.archive/revisions/<revision-id>/` is created and contains the prior certificate state and revision metadata. | The native revision action created one unique revision directory containing `revision.json` and all three archived certificate artifacts. | PASS | Rust `finalized_track_rejects_mutation_until_revision_archives_snapshot`; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 6 | `REQ-WFL-006` | Compare archived files to recorded TD-01 digests. | Archived certificate artifacts match the prior finalized bytes exactly. | Native revision tests snapshotted the three finalized certificate artifacts and asserted exact archived bytes; the workflow-upgrade fixture also compared the archived main hash list byte for byte. | PASS | Rust `finalized_track_rejects_mutation_until_revision_archives_snapshot`; `workflow_upgrade_archives_finalized_v1_and_requires_fresh_v11_outputs` |
| 7 | `REQ-WFL-006` | Inspect current integrity inclusion after revision. | `.archive/` is excluded and the new working state requires fresh documents and hashes. | Explicit reevaluation archived the finalized snapshot, returned the working track to `ACTIVE`, marked documents stale, cleared generated/verified integrity and current certificate state, and removed the working SHA-256 list while preserving the archive. | PASS | Rust `workflow_upgrade_archives_finalized_v1_and_requires_fresh_v11_outputs` |
| 8 | `REQ-WFL-006` | Resolve changes and pass the complete gate again. | A distinct new certificate/revision is created; the old revision remains preserved and superseded as appropriate. | Not run | NOT RUN | — |
| 9 | `REQ-WFL-005` | Load TD-01 with TD-04 current. | UI shows finalized workflow `1.0` and current newer version; old certificate bytes and meaning do not change. | Frontend presentation tests exposed the explicit reevaluation action for an older workflow, including finalized tracks, without mutating certificate state; the native fixture snapshotted old certificate bytes before action. | PASS | Frontend `workflowUpgradePresentation` tests; Rust workflow-upgrade fixture |
| 10 | `REQ-WFL-005` | Choose explicit reevaluation under TD-04. | A new working revision is created; a new certificate requires all current requirements and never rewrites archived `1.0`. | Native explicit reevaluation from `1.0` to test workflow `1.1` created one revision archive with exact old certificate/hash bytes, reset current outputs and status, and rejected a duplicate reevaluation. | PASS | Rust `workflow_upgrade_archives_finalized_v1_and_requires_fresh_v11_outputs` |

## Automated checks

```sh
cd src-tauri
cargo test finalized_track_rejects_mutation_until_revision_archives_snapshot
cargo test workflow_upgrade_archives_finalized_v1_and_requires_fresh_v11_outputs
cd ../frontend
npm test -- --run src/domain/workflow.test.ts
```

Expected Rust evidence is `tests::finalized_track_requires_revision_and_archives_snapshot`. Expected frontend evidence includes the `statuses and finalization` suite in `frontend/src/domain/workflow.test.ts`.

## Verification

The reviewer verifies old/new certificate IDs and versions, exact archive bytes, excluded archive paths, controlled invalidation, and full gate rerun before refinalization.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Untouched reopen and full refinalization after revision remain unexecuted as specified. | High | Product team | Execute steps 1 and 8; corrupt-certificate recovery is covered by a native test but does not replace these steps. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 2–7, 9, and 10 passed; two mandatory steps remain `NOT RUN`.
- Residual risks: Untouched packaged reopen and full refinalization after revision still lack complete acceptance evidence.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Workflow invalidation and revision model](../../def/workflow-model.md#certificate-invalidation-and-revision)
- [Finalizing a track](../../usr/finalizing-a-track.md#create-a-revision-after-a-change)
- [Local persistence and recovery](../../def/persistence.md#backup-and-revision-behavior)
