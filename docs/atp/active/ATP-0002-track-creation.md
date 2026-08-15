<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0002: Track creation and conditional questions

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-15 |
| Executed | 2026-08-13/15 — partial automated execution |
| Requirement | [`REQ-TRK-001` through `REQ-TRK-003`](../../def/track-documentation-model.md#requirements-and-atp-mapping), [`REQ-WFL-002`](../../def/workflow-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current 2026-08-15 working tree not yet committed; package rebuild remains open in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; temporary Rust workspaces plus Vitest workflow, navigation, and delegated-dialog fixtures |

## Purpose

This plan verifies safe track creation, a usable new-track dialog, the standard portable folder skeleton, global-value snapshots, and conditional question behavior.

## Objective

Accept track creation when the modal remains usable while its fields are edited, a unique title produces exactly one contained `DRAFT` track without fake evidence, and the workflow asks dependent questions only when their controlling answer makes them relevant.

## Scope

### Included

- title validation and collision handling;
- new-track modal click routing and field usability;
- required directory creation;
- absence of placeholder media;
- initial lifecycle and ten-step state;
- global defaults copied into mutable track data; and
- `No` and `Yes` branch behavior for source, editing, and artwork questions.

### Excluded

- document generation;
- evidence file content validation; and
- finalization.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Unsafe title becomes a traversal path | Write escapes the workspace | Use separators and traversal strings as negative data |
| Existing track is overwritten | Evidence loss | Pre-create a colliding target and compare hashes |
| Hidden branch remains mandatory | User cannot complete a valid minimal track | Exercise both controlling values and missing-item output |
| Generic editing claims are preselected | Inaccurate documentation | Inspect fresh-track values and branch changes |
| Delegated backdrop click handles an inner field as dismissal | Title/date input is lost and no track can be created | Unit-check direct-versus-descendant routing, then execute the complete field/control flow in the identified GUI build |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] ATP-0001 prerequisites can create a disposable workspace.
- [ ] TD-01 global defaults are saved.
- [ ] Test data contains no sensitive production data.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Global defaults | Artist `Acceptance Artist`, Suno plan `Pro`, commercial intent `Yes`, transparency policy `Always add visible AI disclosure` |
| TD-02 | Valid track | Title `Acceptance Track`, production dates `2026-08-01` through `2026-08-02` |
| TD-03 | Collision | Existing child folder named for TD-02 containing a sentinel text file |
| TD-04 | Unsafe titles | `../escape`, `/absolute`, a separator-only value, and an empty value |
| TD-05 | New-track modal | Empty modal with title, production-start date, commercial-use toggle, close button, cancel action, and backdrop |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-TRK-001` | Create TD-02 in a workspace without TD-03. | One contained track is created with lifecycle `DRAFT` and workflow ID/version. | A temporary track was created as `DRAFT` with workflow `suno-track` `1.0`. | PASS | Rust `track_creation_builds_exact_folders`; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 2 | `REQ-TRK-001` | Inspect the new tree. | `.archive/` and directories `01_RELEASE` through `06_CERTIFICATE` exist in the documented structure. | The Rust test asserted every documented directory in the temporary track. | PASS | Rust `track_creation_builds_exact_folders` |
| 3 | `REQ-TRK-001` | Search the new tree for media, PDF, and ZIP files. | No empty or fake audio, video, image, PDF, or archive evidence exists. | Not run | NOT RUN | — |
| 4 | `REQ-TRK-001` | Open the track facts. | TD-01 defaults appear as editable track values and later document generation can snapshot them. | Not run | NOT RUN | — |
| 5 | `REQ-TRK-002` | Answer `No` to external, own, and third-party audio upload. | Dependent source, ownership, license, and file questions are hidden and excluded from the applicable requirement set. | Vitest confirmed hidden fields and exclusion from the applicable set for negative controllers. | PASS | Frontend `conditional fields` and `progress` suites; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 6 | `REQ-WFL-002` | Change external audio upload to `Yes`. | Source, ownership, license evidence, and uploaded-file requirements appear as missing items. | Vitest produced the four external-source, ownership, file, and license missing items. | PASS | Frontend `lists only applicable missing items` |
| 7 | `REQ-TRK-002` | Answer `No` to human editing and post-export editing. | Specific editing fields are hidden and no generic arrangement, mixing, or mastering claim is selected. | Not run | NOT RUN | — |
| 7a | `REQ-WFL-002` | In Human Work, select a lyrics source, enter the used lyrics and Suno style prompt, then choose multiple confirmed editing steps. | Non-instrumental lyrics and the style prompt are required; confirmed work is stored as the deterministic multi-selection rather than unrestricted prose. | The frontend requires the style prompt, conditionally requires used lyrics, and round-trips de-duplicated guided choices. Native workflow evaluation requires `human_work.suno_style_prompt`. | PASS | Vitest `stores multiple guided choices deterministically`, `lists only applicable missing items`; Rust embedded workflow validation |
| 7b | `REQ-WFL-002` | In Release, choose multiple release-note options. | The field accepts the declared release-version choices and does not present an unrestricted notes textarea. | The release editor uses the same deterministic multi-choice control and retains optional selection semantics. | PASS | Vitest `stores multiple guided choices deterministically`; frontend `renderStepContent` |
| 8 | `REQ-WFL-002` | Declare AI-assisted artwork and then human-only artwork in separate runs. | AI evidence/disclosure requirements appear only for the AI-assisted run; the whole AI Transparency step can be stored as N/A for the human-only run only with a reason. | Not run | NOT RUN | — |
| 9 | `REQ-TRK-001` | Create TD-03, then attempt TD-02 again. | Creation stops with a collision error and the sentinel remains byte-identical. | Not run | NOT RUN | — |
| 10 | `REQ-TRK-001` | Attempt each TD-04 title. | Each invalid title returns a controlled error and no outside or malformed track is created. | Path-like, absolute-looking, traversal, separator-only, and empty titles are rejected before folder creation. | PASS | Rust `track_creation_rejects_path_like_titles_without_writing_folders`; native title validation review |
| 11 | `REQ-TRK-003` | Exercise delegated click routing with a direct modal backdrop target, a descendant target routed through that backdrop, and a non-backdrop close control. | A descendant target is ignored as a backdrop dismissal; a direct backdrop target and explicit controls retain their close actions. | The routing predicate returned `ignore` only for the descendant-through-backdrop case; direct backdrop and non-backdrop action cases remained actionable. Static review confirmed that close and cancel buttons keep their explicit actions. | PASS | Vitest `navigation > ignores delegated backdrop actions for clicks inside a modal`; `frontend/src/app.ts` delegated-click guard |
| 12 | `REQ-TRK-003` | In an identified GUI build, open TD-05, type a title, change the date, and toggle commercial use; repeat separately for direct backdrop, close button, and cancel dismissal. | Every inner click keeps the dialog open and retains all entered values. Each documented dismissal path closes it only when deliberately invoked. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test track_creation_builds_exact_folders
cd ../frontend
npm test -- --run src/domain/workflow.test.ts
npm test -- --run src/app.test.ts
```

Expected Rust evidence is `tests::track_creation_builds_exact_folders`. Expected Vitest evidence includes `conditional fields > hides external-audio details until yes`, `shows own-audio and sample details only when applicable`, `shows AI/artwork follow-ups conditionally`, and `navigation > ignores delegated backdrop actions for clicks inside a modal`.

## Verification

Record the relative tree, initial view model, branch-specific missing items, collision sentinel hash, and automated report. A reviewer maps every expected result above to evidence before approval.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Steps 3, 4, 7–9, and 12 remain unexecuted. | Medium | Product team | Run the remaining placeholder, snapshot, editing/N/A, collision, and complete packaged-GUI dialog fixtures. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 2, 5, 6, 10, and 11 passed; six mandatory steps remain `NOT RUN`.
- Residual risks: The event-routing regression is covered, but real GUI field retention/dismissal, collision sentinels, immutable snapshot details, and editing/N/A branches are not accepted yet.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Workflow model](../../def/workflow-model.md)
- [Getting started](../../usr/getting-started.md)
