<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0008: Finalization gate and missing-item calculation

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-16 |
| Executed | 2026-08-13/15/16 — partial automated execution |
| Requirement | [`REQ-WFL-001`, `REQ-WFL-003`, `REQ-WFL-004`](../../def/workflow-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current 2026-08-16 working tree not yet committed; local `sunodm.AppImage` rebuilt |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; Vitest workflow fixtures and native end-to-end fixture |

## Purpose

This plan verifies the exact ten-step workflow, conditional progress, concrete missing items, blocker states, and native finalization readiness decision.

## Objective

Accept the finalization gate when it exposes every actionable blocker, permits only `PASS` or reasoned `N/A` mandatory outcomes, and cannot be bypassed by frontend state.

## Scope

### Included

- declarative workflow identity, version, and ten-step ordering;
- step status and N/A-reason rules;
- applicable-requirement progress;
- missing evidence, stale document, hash, and deviation blockers; and
- disabled/enabled and native gate behavior.

### Excluded

- certificate content, covered by ATP-0009;
- legal evaluation; and
- arbitrary user-defined workflows.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| UI and native gate disagree | Invalid certificate or confusing block | Compare typed view model and native validation for every fixture |
| N/A without reason clears requirement | Missing documentation is hidden | Attempt empty and whitespace reasons |
| Percentage omits a blocker | User sees false readiness | Seed one blocker of each class at high progress |
| Cached readiness bypasses changed file | Stale snapshot finalized | Change evidence immediately before native invocation |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] The parsed workflow artifact and application version are recorded.
- [ ] Disposable fixtures exist for every step status and conditional branch.
- [ ] No fixture contains sensitive production data.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Empty draft | Only valid track identity exists |
| TD-02 | Minimal applicable track | Negative optional branches excluded from evaluation, one genuinely non-applicable step with a stored N/A reason, and all remaining requirements complete |
| TD-03 | Blocker matrix | Separate `NOT RUN`, `FAIL`, `BLOCKED`, `NOT VERIFIED`, empty N/A reason, missing evidence, stale document, hash failure, and open deviation fixtures |
| TD-04 | Ready track | All applicable mandatory requirements pass, documents current, and hashes verified |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-WFL-001` | Load the declarative workflow. | ID `suno-track`, version `1.1`, and exactly ten uniquely ordered required steps load; unsupported schema or duplicate step is rejected. | The embedded version 1.1 workflow loaded with ID `suno-track`, ten ordered steps, and schema 1; unsupported schema, empty/duplicate step IDs, unknown kinds, missing fields, and a missing mandatory step were rejected. | PASS | Rust workflow configuration tests |
| 2 | `REQ-WFL-003` | Evaluate TD-01. | Concrete missing items identify requirement key, step, user-facing label/reason, and resolution action; progress reflects only completed applicable mandatory items. | Not run | NOT RUN | — |
| 3 | `REQ-WFL-003` | Evaluate TD-02 with each negative conditional controller. | Dependent requirements are excluded from numerator and denominator; explicitly marking a wholly non-applicable step N/A requires a non-empty reason. | Not run | NOT RUN | — |
| 4 | `REQ-WFL-004` | Attempt manual N/A with an empty or whitespace reason. | Validation rejects it and finalization remains blocked. | Not run | NOT RUN | — |
| 5 | `REQ-WFL-004` | Evaluate every TD-03 status fixture. | Each `NOT RUN`, `FAIL`, `BLOCKED`, and `NOT VERIFIED` mandatory result appears as an exact blocker. | Not run | NOT RUN | — |
| 5a | `REQ-WFL-004` | Leave any preceding step incomplete and inspect `10 Finalize`. | Finalize cannot display a completion check while an earlier mandatory step is open. | Both native and frontend status evaluation return `BLOCKED` for Finalize until every preceding step is `PASS` or justified `N/A`. | PASS | Rust `finalize_is_blocked_while_a_preceding_step_is_incomplete`; Vitest `keeps Finalize blocked until every preceding step is complete` |
| 5b | `REQ-WFL-004` | Fulfill a legacy step that has a stored `NOT VERIFIED` result, then reevaluate. | The historical blocker is replaced by `PASS`; it cannot permanently block an otherwise complete Track, Suno, Integrity, or Finalize step. | Unit evaluation promoted a fulfilled legacy Track step, and the reopen integration promoted Track, Suno, Integrity, and Finalize before passing native validation. | PASS | Rust `fulfilled_legacy_step_recovers_from_stored_not_verified_status`; `reopening_assigns_saved_global_profile_to_existing_legacy_track` |
| 6 | `REQ-WFL-004` | Evaluate missing evidence, stale document, hash failure, and open deviation fixtures. | Each independently blocks readiness even if all other steps pass. | Not run | NOT RUN | — |
| 7 | `REQ-WFL-003` | Compare progress before and after making an optional branch applicable. | Denominator and missing items update deterministically; optional nonmandatory values do not lower mandatory progress. | Vitest asserted both denominator change for an applicable branch and exclusion of non-applicable requirements. | PASS | Frontend `progress` suite; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 8 | `REQ-WFL-004` | Evaluate TD-04. | Lifecycle is `READY`, no blocker remains, and the Finalize action is enabled. | The complete frontend fixture enabled its gate, and native validation returned `valid` with no missing or blocking item. | PASS | Frontend `allows finalization only when every applicable requirement is complete`; Rust end-to-end test |
| 9 | `REQ-WFL-004` | Change one protected TD-04 file after UI readiness, then invoke finalization directly. | Native reevaluation rejects finalization and identifies the new integrity blocker. | Not run | NOT RUN | — |
| 10 | `REQ-WFL-004` | Restore, regenerate, and verify TD-04, then invoke the native gate. | Gate succeeds and hands the validated snapshot to certificate creation exactly once. | The native integration path regenerated documents, calculated hashes, passed `validate_track`, invoked finalization once, and received a valid certificate. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate` |

## Automated checks

```sh
cd src-tauri
cargo test workflow::tests
cargo test blocking_deviation_prevents_validation_and_finalization_until_resolved
cd ../frontend
npm test -- --run src/domain/workflow.test.ts
```

Expected Rust evidence is `tests::finalization_gate_rejects_missing_and_blocking_deviation`. Expected Vitest evidence includes `missing requirements > lists only applicable missing items`, `requires source, ownership, file and license on positive source branches`, both `progress` cases, `statuses and finalization > renders/derives NOT_RUN/PASS/FAIL/BLOCKED/N_A`, `blocks finalization for missing evidence, stale documents, hash mismatch and unresolved deviation`, and `allows finalization only when complete`.

## Verification

The reviewer maps every documented blocker class to a failing fixture and confirms that only TD-04 can cross the native gate. A 100 percent UI value without native success is not accepted.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Full blocker/status/N/A matrices and the post-readiness changed-file race were not executed as specified. | High | Product team | Execute steps 2–6 and 9 with native/frontend parity assertions. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 7, 8, and 10 passed; six mandatory steps remain `NOT RUN`.
- Residual risks: Complete status/blocker parity and time-of-check/time-of-use protection lack acceptance evidence.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Workflow model](../../def/workflow-model.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
- [ATP-0009: Certificate generation](ATP-0009-certificate-generation.md)
