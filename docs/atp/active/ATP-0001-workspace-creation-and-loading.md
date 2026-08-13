<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0001: Workspace creation and loading

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-13 |
| Executed | 2026-08-13 — partial automated execution |
| Requirement | [`REQ-PER-001`](../../def/persistence.md#requirements-and-atp-mapping), [`REQ-ARC-003`](../../def/product-architecture.md#product-requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; unversioned source tree; Linux package digests in the central report |
| Environment | Linux `7.0.8-1-cachyos` `x86_64`; Rust core tests and Chromium visual smoke review |

## Purpose

This plan verifies that a user can create, close, and reopen a contained local workspace without a server or unrestricted filesystem access.

## Objective

Accept workspace onboarding when the application initializes its management area and SQLite schema only inside the selected root, restores saved non-sensitive settings, and handles cancel, invalid path, and existing-workspace cases without data loss.

## Scope

### Included

- no-workspace first-launch state;
- native create and select actions;
- `.suno-doc/` initialization and SQLite reopening;
- minimal global settings persistence;
- cancel and invalid-selection behavior; and
- offline operation.

### Excluded

- track creation;
- remote or synchronized workspaces; and
- operating-system permission installation.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Management files escape the selected root | Unintended filesystem modification | Compare surrounding tree and canonical paths before and after creation |
| Reopening replaces existing settings | Local data loss | Seed known settings, close cleanly, and reopen the same root |
| Product requires a network service | Offline use fails | Disable network access for the runtime path |
| Expected path error panics the process | Unsaved work and poor recovery | Exercise invalid file and read-only destination cases |

## Preconditions

- [ ] Required desktop dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] The application starts in a clean user-data context.
- [ ] Disposable local test folders are prepared and contain no sensitive production data.
- [ ] Network access can be disabled without preventing the packaged application from launching.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Empty writable workspace parent | Create a new temporary directory using the platform test harness |
| TD-02 | Existing empty workspace directory | Create a second temporary directory before launch |
| TD-03 | Invalid workspace target | Create a normal file where a directory is expected |
| TD-04 | Minimal global profile | Artist `Acceptance Artist`, profile `acceptance-profile`, plan `Pro`, date `2026-08-01`, no private contact data |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-PER-001` | Launch with no previously selected workspace. | The UI states that no workspace is open and offers native select and create actions. | Welcome view and both actions were present in the responsive Chromium smoke review. | PASS | [Central execution report](../../dev/acceptance-report.md) |
| 2 | `REQ-PER-001` | Cancel the native create action. | No workspace is selected and no folder or database is created. | Not run | NOT RUN | — |
| 3 | `REQ-PER-001` | Create a workspace below TD-01. | The selected root opens and contains one `.suno-doc/` management area with a usable `workspace.sqlite`. | A temporary workspace opened with a usable `.suno-doc/workspace.sqlite` and zero initial tracks. | PASS | Rust `workspace_creation_initializes_local_database`; [final suite](../../../.report/test-report-20260813-144834-suite-all-ok.md) |
| 4 | `REQ-ARC-003` | Compare TD-01's parent and sibling trees before and after creation. | No product-managed file is written outside the selected workspace. | Not run | NOT RUN | — |
| 5 | `REQ-PER-001` | Save TD-04 in settings, close the app, disable network access, and reopen the same workspace. | The app opens offline and restores all saved values exactly. | Not run | NOT RUN | — |
| 6 | `REQ-PER-001` | Inspect stored and rendered fields. | No birthday, telephone, private email, credentials, or unrelated account fields are requested or introduced. | Static model/form scan found only the documented profile facts; the frontend profile test asserts the required field set. | PASS | Frontend `missingProfileFields`; [central report](../../dev/acceptance-report.md) |
| 7 | `REQ-PER-001` | Select TD-02 as an existing workspace. | The management area is initialized only after confirmation and the workspace becomes current. | Not run | NOT RUN | — |
| 8 | `REQ-ARC-003` | Attempt to open TD-03. | A controlled user-readable error is shown; the process remains usable and the file is unchanged. | Not run | NOT RUN | — |
| 9 | `REQ-PER-001` | Switch back to the first workspace. | Its distinct settings are restored and no values leak from TD-02. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test workspace_creation_initializes_local_database
cd ../frontend
npm test -- --run src/api/desktop.test.ts
```

Expected Rust evidence is `tests::workspace_creation_initializes_local_database`. Expected frontend evidence includes `toUserMessage > presents structured/string/unknown errors` and `isTauriRuntime > distinguishes browser demo`.

## Verification

The acceptance owner records screenshots of the first-launch and reopened states, a relative file-tree listing, and the automated report. Evidence must not contain the machine's absolute workspace path or real personal data.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Native cancellation, offline restart, invalid-target, tree-boundary, and workspace-switching steps remain unexecuted. | Medium | Product team | Execute steps 2, 4, 5, and 7–9 in the packaged app. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 3, and 6 passed; six mandatory desktop acceptance steps remain `NOT RUN`.
- Residual risks: Native dialog cancellation and offline restart/isolation are not yet demonstrated.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Getting started](../../usr/getting-started.md)
- [Local persistence and recovery](../../def/persistence.md)
- [Product architecture](../../def/product-architecture.md)
