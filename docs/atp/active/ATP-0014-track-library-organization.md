<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0014: Track library album and single organization

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-14 |
| Last review | 2026-08-14 |
| Executed | 2026-08-14 — partial automated execution; packaged GUI steps not run |
| Requirement | [`REQ-LIB-001` and `REQ-LIB-002`](../../def/track-library-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; current implementation commit `87b9338b24ecfda31f5abec97d42747aeef91d23` (`🌲 Make track library folders collapsible`), based on library implementation `65a43673b14411463b360ff91e92365cd5347a9a`; no package was built from the current commit |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; Node 26.4.0/npm 12.0.1; rustc/cargo 1.97.1; native temporary workspaces and pure TypeScript fixtures |

## Purpose

This plan verifies that the track library has a stable album-and-single hierarchy while library reclassification remains separate from portable track content and finalization state.

## Objective

Accept the library organization when every indexed track appears exactly once under `Albums/<album title>` or `Singles`, older records remain visible as singles, assignments persist, and moving a track between groups cannot mutate its portable documentation snapshot.

## Scope

### Included

- permanent `Albums` and `Singles` sections;
- named album grouping, sorting, search, and status filters;
- assignment during track creation;
- later single-to-album, album-to-single, and album-to-album reclassification;
- finalized-track reclassification invariants;
- older JSON and scanned-track defaults; and
- typed TypeScript/Tauri/native command mapping.

### Excluded

- independent album records, empty albums, cover art, release metadata, and manual track sequencing;
- physical album or single directories;
- album-level documents, hashes, manifests, or certificates; and
- accepting the interaction from source-level unit tests alone.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| A track appears in both sections or disappears | The library is ambiguous or incomplete | Classify once before filters and assert exact input-ID coverage |
| Case variants split one album | Users see duplicate logical albums | Normalize comparison keys while retaining a display title |
| An invalid album assignment is persisted | A track has no usable parent group | Validate the enum and require a trimmed 1–200-character title without control characters |
| Reclassification uses the normal content-edit path | A finalized certificate or portable snapshot is changed unnecessarily | Use a narrow native command and compare protected state and the complete track tree |
| Older records fail to deserialize | Existing workspaces become unusable | Apply an explicit `single` default without a relational schema change |
| Unit fixtures or static disclosure review mask interaction defects | The shipped folder or dialog workflow cannot be completed by mouse or keyboard | Keep packaged GUI and accessibility steps `NOT RUN` until executed |

## Preconditions

- [x] Required source-test dependencies are installed.
- [x] Implementation commit and source environment are identified.
- [x] Native temporary workspaces contain only synthetic data.
- [ ] A packaged desktop artifact built from the tested implementation is identified.
- [ ] A clean disposable GUI workspace is prepared for mouse, keyboard, responsive, and restart checks.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Album library | `Northern Lights`/case variant with multiple sortable tracks and another named album |
| TD-02 | Singles library | Current single, old record with no `library` JSON field, and a scanned historical track |
| TD-03 | Validation boundary | Empty, whitespace-only, control-character, 200-character, and 201-character album titles |
| TD-04 | Finalized reclassification | Synthetic finalized record with fixed path, timestamp, workflow, document, integrity, certificate, and track-tree sentinel state |
| TD-05 | UI filters | Album and single tracks across `DRAFT`, `ACTIVE`, `READY`, and `FINALIZED` |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-LIB-002` | Submit TD-03 through the native library normalizer and serialize the accepted values. | `single` drops an irrelevant title; `album` trims and accepts 1–200 Unicode characters, rejects a missing/empty/overlong/control-character title, and rejects an unknown section. | Native validation rejected every invalid album case before track creation, normalized singles, trimmed a valid album title, and serialized the default as `{"section":"single"}`. The frontend helper matched the same boundaries, including a 200-character title surrounded by whitespace. | PASS | Rust `track_library_validation_rejects_invalid_albums_and_normalizes_singles`; Vitest `track library assignment` |
| 2 | `REQ-LIB-002` | Create an album track, close the application object, reopen it, then load TD-02 fixtures. | Create, detail, and summary retain the normalized assignment; old JSON and scanned tracks load as singles; the schema stays version 2 and scan changes no track file. | The album assignment survived reopen and list/detail conversion. Missing JSON defaulted and later materialized as `single`; a historical folder scanned as `single` with an identical before/after file-tree snapshot. `SCHEMA_VERSION` remained 2. | PASS | Rust `track_creation_persists_album_library_placement_after_reopen`, `older_track_json_defaults_to_single_library_section`, `legacy_scan_defaults_library_placement_without_modifying_track_files` |
| 3 | `REQ-LIB-002` | Reclassify TD-04 from single to a trimmed album and back to single. | Only `library` changes. Path, timestamps, lifecycle/workflow/profile/field/document/integrity/certificate state and every portable track byte remain unchanged; no finalized-track restriction or revision is introduced. | Both directions succeeded while status remained `FINALIZED`. Comparing serialized track records after removing only `library` and comparing the complete track tree showed equality; `updated_at`, document, integrity, and certificate sentinels remained intact. Static review confirmed the narrow path only reads related step/evidence/deviation rows. | PASS | Rust `library_reclassification_preserves_finalized_track_state_and_files`; reviewed `update_track_library` command path; Vitest demo reclassification invariant |
| 4 | `REQ-LIB-001` | Group mixed TD-01 and TD-02 input, including case variants and an invalid/missing legacy assignment. | Both top-level sections are always returned; case variants share one album; album groups/tracks/singles sort deterministically; every input ID appears exactly once and invalid legacy presentation falls back to `Singles`. | Pure TypeScript fixtures returned both empty sections, grouped `Neon Nights` case-insensitively, sorted groups and rows, defaulted missing/invalid album presentation to singles, and asserted exact unique ID coverage. | PASS | Vitest `track library grouping` permanent-sections, sorting, and exact-once cases |
| 5 | `REQ-LIB-001` | Search TD-05 by album title and track path, then apply the `Open` status filter to mixed album and single rows. | Search preserves the hierarchy; an album-title match returns its eligible tracks; a path match returns only matching rows; the filter acts inside both sections. | Album-title and path fixtures retained the expected album/single parents, while the open-status fixture returned only `DRAFT`/`ACTIVE` rows from both sections and excluded its `READY`/`FINALIZED` rows. | PASS | Vitest `searches track paths and album titles while retaining the hierarchy`; `applies the status filter inside album and single groups` |
| 6 | `REQ-LIB-001`, `REQ-LIB-002` | Exercise the pure assignment helper, summary update, demo API, and desktop adapter, then inspect the source-level folder disclosure structure. | Create sends the selected placement; reclassification invokes only `update_track_library`; returned placement updates the in-memory summary; demo behavior preserves non-library state; top-level sections and named albums use nested native disclosures that start expanded. | Adapter assertions matched the exact `create_track` and `update_track_library` payloads, the summary retained the returned album placement, and demo reclassification changed no other detail field. Static review found nested `details`/`summary` nodes for `Alben`, each album, and `Singles`, with track lists inside their disclosure content. | PASS | Vitest `desktop.test.ts`, `app.test.ts`, and `demo.test.ts`; reviewed `renderTracks`/`renderAlbumGroup`; [full-suite report](../../../.report/test-report-20260814-134811-suite-all-ok.md), SHA-256 `d7779bce7ccf72b336b960c781ab0ca8fd0852543f4fb9ca2ff5fdcb3122ec08` |
| 7 | `REQ-LIB-001`, `REQ-LIB-002` | In a packaged app, create one single and two album tracks; collapse and expand `Alben`, every album row, and `Singles`; click and type inside every dialog field; search/filter; finalize one track; and reclassify it in every direction. | Pointer and keyboard activation reveal and hide only the correct descendants, disclosure indicators follow the state, and tracks remain nested below their album or `Singles`. Dialogs remain open during field interaction; conditional input, suggestions, hierarchy, empty states, and saved assignments are correct; finalized reclassification leaves the visible path/status/certificate unchanged. | Not run | NOT RUN | — |
| 8 | `REQ-LIB-001`, `REQ-LIB-002` | Restart the packaged app, reopen the workspace, then review the library and both dialogs at narrow/wide viewport sizes using keyboard and a screen reader. | Assignments persist; layout remains usable; dialog focus enters and stays within the modal, `Escape` closes it, focus returns to the trigger, and names/instructions are announced. | Not run | NOT RUN | — |

## Automated checks

Run from the repository root:

```sh
cd src-tauri
cargo test --locked track_library
cargo test --locked library_reclassification
cargo test --locked older_track_json
cargo test --locked legacy_scan_defaults_library
cd ../frontend
npm test -- --run
npm run build
cd ..
python tools/control.py test --suite all --report
```

The repository-wide command passed at current implementation commit `87b9338b24ecfda31f5abec97d42747aeef91d23`: tools reported 177 passed/21 skipped, frontend reported 39 passed in six files, schema validation passed, and the Tauri structure, Cargo check, and Rust tests passed. The complete Rust run for the underlying library implementation separately reported 88 passed, 0 failed, and one opt-in removable-filesystem test ignored by default.

## Verification

Review the native record/tree assertions, TypeScript exact-once grouping, adapter payloads, source-test report, and schema version. Do not promote steps 7 or 8 from unit or static evidence. A package built from an older commit does not accept this feature.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | No package was built from `87b9338b…`; real folder disclosure and modal interaction, persisted restart, responsive rendering, initial dialog focus, focus containment/restoration, `Escape`, and screen-reader behavior were not executed. | High | Product team | Build a clean current package and execute steps 7 and 8 with retained screenshots and keyboard/screen-reader notes. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1–6 passed through identified source automation and review; packaged GUI and restart/accessibility steps 7–8 remain `NOT RUN`.
- Residual risks: The native organization contract is covered, but the complete shipped dialog workflow and keyboard accessibility are not yet accepted.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track library organization model](../../def/track-library-model.md)
- [Getting started](../../usr/getting-started.md)
- [Product architecture](../../def/product-architecture.md)
- [Persistence and recovery](../../def/persistence.md)
- [Central acceptance report](../../dev/acceptance-report.md)
