<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0013: End-to-end offline track workflow

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-14 |
| Executed | 2026-08-13/14 — partial automation plus packaged offline launch/dialog observations; complete GUI workflow not finished |
| Requirement | [`REQ-ARC-001`](../../def/product-architecture.md#product-requirements-and-atp-mapping) and the integrated requirements referenced by ATP-0001 through ATP-0012 |
| Tested commit/build | Product `0.1.0`; retained DEB/RPM artifacts identify stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571`; modal/subscription regression implementation `b7e9797b277f0bcac58d4503049002e354cb93fb` is not yet rebuilt as a package |
| Environment | Linux `7.1.4-arch1-1` host; disposable Debian 13.2 KDE/Wayland VM with NIC down; installed DEB plus native core/frontend tests |

## Purpose

This plan verifies the required complete user journey in one identified packaged build while all network access is unavailable.

## Objective

Accept the version 0.1 integrated product when a user can create a workspace and profile, document a track, import evidence, generate documents and AI disclosure, verify SHA-256, finalize, inspect the certificate, close, reopen, and independently review the portable folder without a backend or internet connection.

## Scope

### Included

- one happy-path integration across every product service;
- German desktop navigation and controlled error presentation;
- offline runtime behavior;
- portable result after application restart; and
- absence of backend, server, PostgreSQL, telemetry, and remote calls.

### Excluded

- exhaustive negative cases owned by focused ATPs;
- installer download while offline; and
- legal approval of the synthetic track.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Individually tested services fail in sequence | Version 0.1 user outcome is not delivered | Execute one uninterrupted full flow with timestamps and artifacts |
| Hidden network request blocks an action | Offline requirement fails | Disable network before launch and monitor attempted connections |
| Browser-only fallback masks native failure | Acceptance does not represent product | Use identified packaged Tauri artifact |
| Certificate exists but folder is not portable | Recovery goal fails | Reopen after index isolation in a fixture copy and inspect without app |

## Preconditions

- [ ] The packaged desktop artifact, commit, version, and target OS are identified.
- [ ] Installation and all dependencies are complete before network isolation.
- [ ] Network interfaces or an equivalent controlled deny rule are active for the entire runtime test.
- [ ] A clean temporary workspace and synthetic evidence set are prepared.
- [ ] Focused ATP deviations relevant to this path are reviewed before execution.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Global profile | Synthetic artist/Suno values, plan and subscription dates, default AI policy |
| TD-02 | Track facts | `Offline Acceptance Track`, fixed production/export dates, explicit conditional answers and human work |
| TD-03 | Evidence set | Small valid synthetic release WAV, Suno evidence, one monthly or annual subscription document with factual coverage start, AI original, and required final files |
| TD-04 | AI disclosure | Default `AI-assisted` text and placement on synthetic artwork |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-ARC-001` | Disable network, launch the identified packaged app, and observe connections. | App starts with no backend, local HTTP sidecar, telemetry, remote API, or required connection. | Not run | NOT RUN | — |
| 2 | ATP-0001 | Create a new workspace and save TD-01. | Local management area initializes and settings persist. | The native integration fixture created the workspace, initialized SQLite, and saved a complete synthetic profile. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate`; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 3 | ATP-0002 | Create TD-02 and complete conditional questions truthfully. | Standard tree exists, only relevant questions appear, and concrete missing items update. | Native creation produced the standard tree; frontend fixtures verified relevant conditional fields and concrete missing-item updates. | PASS | Rust end-to-end plus frontend `conditional fields`/`missing requirements` suites |
| 4 | ATP-0005/0011 | Import TD-03 through native pickers. For the subscription document, choose its monthly/annual cadence, enter the factual start, review the materialized end, and select exactly one supported file. | Every source remains unchanged; contained copies, roles, sizes, hashes, and workflow reevaluation succeed. The subscription record stores one exact interval and does not imply later recurring evidence. | Not run | NOT RUN | — |
| 5 | ATP-0004 | Generate all required documents. | Versioned factual outputs exist, contain track snapshots, and have no legal guarantee. | The native path generated and freshness-checked the complete eight-file version `1.0` document set. | PASS | Rust end-to-end test; static document disclaimer review |
| 6 | ATP-0006 | Generate TD-04 locally. | AI original remains unchanged, separate visible output exists, and process documents are current. | The expanded native integration imported an AI original, generated a distinct verified `AI_EDITED` disclosure copy, preserved the source, imported final artwork, and then freshness-checked all process documents. | PASS | Rust end-to-end and artwork-disclosure preservation tests |
| 7 | ATP-0007 | Generate and verify the main SHA-256 list. | Included/excluded sets are correct, generated and verified counts match, and integrity is `PASS`. | The integrated native action generated, reread, and verified the current exact set before finalization. | PASS | Rust end-to-end and integrity exact-set tests |
| 8 | ATP-0008 | Review readiness and invoke finalization. | Every mandatory step passes or has reasoned N/A, no blocker remains, and native gate succeeds. | Native validation returned no missing/blocking item and finalization succeeded. | PASS | Rust `end_to_end_documentation_workflow_creates_portable_certificate` |
| 9 | ATP-0009 | Inspect certificate artifacts. | Certificate, relative-path manifest, and certificate hash list are complete, factual, and internally verified. | All three certificate artifacts were created; native certificate verification passed and the manifest contained portable evidence paths without the workspace root. | PASS | Rust end-to-end and certificate parser tests |
| 10 | `REQ-ARC-001` | Close and reopen the same workspace while still offline. | Finalized status and artifacts load and verify without network access. | Not run | NOT RUN | — |
| 11 | `REQ-ARC-004` | Copy the portable track folder to a fresh fixture location and inspect it without the app or original index. | Folder remains human-readable; root-relative hash checks and manifest references remain usable. | Not run | NOT RUN | — |
| 12 | `REQ-ARC-001` | Review runtime logs/processes and repository product units. | No FastAPI, backend service, PostgreSQL, remote database, cloud sync, telemetry, or local HTTP dependency participated. | Not run | NOT RUN | — |

## Automated checks

Run before or after the isolated manual path from the repository root; they do not replace the packaged offline run:

```sh
cd src-tauri
cargo test end_to_end_documentation_workflow
cd ..
python tools/control.py test --suite all --report
python tools/control.py build web
python tools/control.py build desktop
python tools/control.py release check
```

Expected Rust evidence is `tests::end_to_end_documentation_workflow`. Expected frontend evidence includes every suite in `frontend/src/domain/workflow.test.ts` and `frontend/src/api/desktop.test.ts`. If `frontend/src/app.test.ts` is present in the identified build, also attach `navigation > reaches every main view and all ten workflow steps` and `finalization controls > remains disabled until native gate-ready state`. The packaged offline path remains mandatory integration evidence and cannot be replaced by unit tests.

## Verification

Attach the identified artifact, network-isolation method, connection observation, synthetic relative file tree, service-step evidence, both integrity results, restart result, and portable-folder review. Do not attach absolute local paths or private data.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | The installed DEB launched with the VM NIC down and native workspace/evidence dialogs plus restart were observed, but the mandatory uninterrupted GUI track flow, copied-folder verification, and complete runtime observation were not finished. | High | Product team | Repeat steps 1, 4, and 10–12 as one clean finalized workflow with retained evidence. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Core steps 2, 3, and 5–9 passed; five mandatory packaged/offline steps remain `NOT RUN`.
- Residual risks: The packaged app starts offline, but the full packaged happy path and portable-copy proof remain open.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Product README](../../../README.md)
- [Getting started](../../usr/getting-started.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
- [Product architecture](../../def/product-architecture.md)
