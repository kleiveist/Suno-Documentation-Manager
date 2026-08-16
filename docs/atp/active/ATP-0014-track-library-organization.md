<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0014: Track library album and single organization

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-14 |
| Last review | 2026-08-16 |
| Executed | 2026-08-14 — partial automated execution; packaged GUI steps not run |
| Requirement | [`REQ-LIB-001`, `REQ-LIB-002`, and `REQ-LIB-003`](../../def/track-library-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; physical-folder implementation `3d003bf1389cb0d737c047c1ddd5b7a57f4bf448` (`📁 Manage albums and singles as physical folders`), based on collapsible-tree commit `87b9338b24ecfda31f5abec97d42747aeef91d23`; no package was built from the current implementation |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; Node 26.4.0/npm 12.0.1; rustc/cargo 1.97.1; native temporary workspaces and pure TypeScript fixtures |

## Purpose

This plan verifies that the track library has a stable album-and-single hierarchy whose UI, SQLite paths, and physical workspace folders stay synchronized without changing bytes inside a moved track root.

## Objective

Accept the library organization when the workspace always has `Singles/`, users can create and retain an empty physical album from the `Alben` header, every indexed track appears exactly once under `Albums/<album title>` or `Singles`, folder creation and renaming are collision-safe, stale paths recover conservatively, and moving a complete track root cannot mutate its portable documentation snapshot.

## Scope

### Included

- permanent `Albums` and `Singles` sections;
- direct empty-album creation, listing, restart persistence, and rename;
- named album grouping, sorting, search, and status filters;
- assignment during track creation;
- later single-to-album, album-to-single, and album-to-album reclassification;
- physical parent/track-folder creation, track-title rename, and album rename;
- managed external-rename recovery and stale legacy-path repair;
- finalized-track reclassification invariants;
- older JSON and scanned-track defaults; and
- typed TypeScript/Tauri/native command mapping.

### Excluded

- independent album database records, cover art, release metadata, and manual track sequencing;
- album-level documents, hashes, manifests, or certificates; and
- accepting the interaction from source-level unit tests alone.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| A track appears in both sections or disappears | The library is ambiguous or incomplete | Classify once before filters and assert exact input-ID coverage |
| Case variants split one album | Users see duplicate logical albums | Normalize comparison keys while retaining a display title |
| An invalid album assignment is persisted | A track has no usable physical parent | Validate the enum and require a safe trimmed 1–200-character folder title |
| An empty physical album disappears from the UI or is scanned as a track | Users cannot prepare the intended folder hierarchy safely | List album directories independently of tracks and assert a zero-track scan result |
| A hidden management folder is rendered as an album or indexed as a track | Archives, tool metadata, or private working state leaks into the library | Prune every leading-dot folder before metadata reads, traversal, identity recovery, and presentation; filter older indexed hidden paths |
| A move overwrites or separates filesystem and SQLite state | Track data is lost or the managed path becomes unusable | Reject destination collisions, transactionally update member paths, and test compensating moves |
| Reclassification uses the normal content-edit path | A finalized certificate or portable snapshot is changed unnecessarily | Use a narrow native move command and compare protected state plus the complete track tree at its new root |
| An external rename leaves a stale index path | Loading fails with `managed path does not exist` | Resolve managed identity markers and test the reported legacy single-candidate repair case |
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
| TD-03 | Validation boundary | Empty, whitespace-only, control-character, separator, reserved, 200-character, and 201-character album titles |
| TD-04 | Finalized reclassification | Synthetic finalized record with fixed timestamp, workflow, document, integrity, certificate, and track-tree sentinel state |
| TD-05 | UI filters | Album and single tracks across `DRAFT`, `ACTIVE`, `READY`, and `FINALIZED` |
| TD-06 | Physical rename | `Gravity Drift/Gravaty`, a second album member, a single, stable identity markers, and byte sentinels |
| TD-07 | Reported stale legacy path | SQLite path `Neuer Ordner` with assigned album `Gravity Drift` and the actual folder `Gravity Drift/Gravaty` |
| TD-08 | Empty physical album | Fresh workspace, `Gravity Drift/` without tracks, permanent `Singles/`, case-variant collision, rename, scan, and reopen |
| TD-09 | Hidden workspace folders | `.archive/Archived Track`, `.draft`, `.cache`, one visible album, and a pre-fix SQLite record below `.archive/` |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-LIB-002`, `REQ-LIB-003` | Submit TD-03 through the native library normalizer and serialize accepted values. | `single` drops an irrelevant title; `album` trims and accepts safe 1–200-character titles and rejects missing, empty, overlong, control-character, separator, traversal, reserved, and unknown values. | Native validation rejected invalid cases before track creation. Frontend validation matched length, control-character, separator, and reserved-name boundaries. | PASS | Rust `track_library_validation_rejects_invalid_albums_and_normalizes_singles`; Vitest `track library assignment` |
| 2 | `REQ-LIB-002` | Create album and single tracks, close and reopen, then load TD-02 fixtures. | Physical paths are `<album>/<track>` and `Singles/<track>`; assignment and path survive reopen; old JSON defaults safely; historical scan remains read-only. | `Night Drive/Album Track` and the permanent Singles parent were created through contained native operations. Assignment/path survived reopen; missing JSON defaulted and materialized; historical candidate bytes and tree stayed identical. | PASS | Rust `track_creation_builds_exact_folders`, `track_creation_persists_album_library_placement_after_reopen`, `older_track_json_defaults_to_single_library_section`, `legacy_scan_defaults_library_placement_without_modifying_track_files` |
| 2a | `REQ-LIB-001`, `REQ-LIB-002`, `REQ-LIB-003` | Create TD-08 through the action in the `Alben` summary, scan, rename it while empty, close, and reopen. | `Singles/` exists; the empty album is immediately visible, scan reports no false track, collision is rejected, rename works, and the folder remains after restart. | Native integration created `Gravity Drift/` and `Singles/`, scanned zero tracks, rejected a case-variant duplicate, renamed the empty folder, and listed it after reopening. Pure TypeScript grouping retained and searched zero-track albums; desktop and demo adapters covered list, create, and empty rename. | PASS | Rust `album_creation_persists_an_empty_folder_and_supports_rename`; Vitest `keeps physical albums visible before their first track is created`, `maps physical album listing and creation to narrow native commands`, `creates and renames an album before it contains a track` |
| 2b | `REQ-LIB-001`, `REQ-LIB-002`, `REQ-LIB-003` | Open, scan, and reopen TD-09; attempt to create `.archive` as an album. | No leading-dot folder is read as an album or track, no warning is emitted for its contents, a pre-fix hidden record stays unavailable and untouched, the visible album still loads, and leading-dot album names are rejected. | Native discovery returned only the visible album track, did not persist hidden candidates, and kept a pre-fix hidden identity intact across reopen. Native and frontend validation rejected leading-dot albums; presentation discarded hidden physical-album and track inputs. | PASS | Rust `hidden_workspace_folders_are_pruned_from_album_and_track_discovery`, `previously_indexed_hidden_paths_remain_unloaded_after_reopen`, `track_library_validation_rejects_invalid_albums_and_normalizes_singles`; Vitest `does not load hidden folders or tracks into the rendered library`, `track library assignment` |
| 3 | `REQ-LIB-002` | Reclassify TD-04 from single to album and back, then force a SQLite uniqueness failure after a physical move. | Relative path follows successful moves; protected state and every byte below the track root remain unchanged; a successful move retains the now-empty source album, while a persistence failure restores the source path and removes only the attempted parent. | Both finalized moves preserved the complete tree and protected state, and the successful source album remained reusable. The forced database rejection returned a controlled error, restored `Singles/First`, removed the attempted album path, and left the original SQLite record unchanged. | PASS | Rust `library_reclassification_preserves_finalized_track_state_and_files`, `library_move_rolls_back_when_the_database_rejects_the_new_path`; Vitest demo reclassification path/state test |
| 4 | `REQ-LIB-001` | Group mixed TD-01 and TD-02 input, including case variants and an invalid/missing legacy assignment. | Both top-level sections are always returned; case variants share one album; album groups/tracks/singles sort deterministically; every input ID appears exactly once and invalid legacy presentation falls back to `Singles`. | Pure TypeScript fixtures returned both empty sections, grouped `Neon Nights` case-insensitively, sorted groups and rows, defaulted missing/invalid album presentation to singles, and asserted exact unique ID coverage. | PASS | Vitest `track library grouping` permanent-sections, sorting, and exact-once cases |
| 5 | `REQ-LIB-001` | Search TD-05 by album title and track path, then apply the `Open` status filter to mixed album and single rows. | Search preserves the hierarchy; an album-title match returns its eligible tracks; a path match returns only matching rows; the filter acts inside both sections. | Album-title and path fixtures retained the expected album/single parents, while the open-status fixture returned only `DRAFT`/`ACTIVE` rows from both sections and excluded its `READY`/`FINALIZED` rows. | PASS | Vitest `searches track paths and album titles while retaining the hierarchy`; `applies the status filter inside album and single groups` |
| 6 | `REQ-LIB-001`, `REQ-LIB-002`, `REQ-LIB-003` | Exercise album-list/create, track-create/reclassification, and album-rename adapters and demo paths, then inspect disclosures and album-header controls. | Commands use typed payloads; returned physical paths and empty albums update presentation; the `Alben` summary exposes a separate create control, and album headers remain collapsible with rename controls. | Vitest covered exact `list_albums`, `create_album`, `create_track`, `update_track_library`, and `rename_album` mappings, demo physical path changes, empty album state, summary updates, and the permanent nested disclosure structure. | PASS | Vitest `desktop.test.ts`, `app.test.ts`, and `demo.test.ts`; reviewed `renderTracks`/`renderAlbumGroup` |
| 7 | `REQ-LIB-003` | Rename TD-06 through the album-header action, rename a track title, rename an album externally, and reopen TD-07. | All members move, byte sentinels survive, SQLite paths follow, managed identity reconnects external renames, and the reported stale legacy record loads from `Gravity Drift/Gravaty`. | Native tests moved both album members, preserved sentinels, renamed a single leaf, recovered an external managed album rename, and repaired `Neuer Ordner` to `Gravity Drift/Gravaty` with display title `Gravaty`. | PASS | Rust `album_rename_moves_the_folder_and_updates_every_member_path`, `changing_a_track_title_renames_its_managed_folder`, `reopen_recovers_an_externally_renamed_album_folder_from_track_identity`, `reopen_repairs_the_reported_legacy_missing_path_from_its_album_folder` |
| 8 | `REQ-LIB-001`, `REQ-LIB-002`, `REQ-LIB-003` | In a packaged app, create one single and two album tracks; collapse/expand folders; rename an album; reclassify a finalized track; restart; and review at narrow/wide viewport sizes with keyboard and a screen reader. | Physical folders and visible paths match; dialog interaction, rename control, collision feedback, focus behavior, persistence, layout, and announcements are correct. | Not run | NOT RUN | — |

## Automated checks

Run from the repository root:

```sh
cd src-tauri
cargo test --locked track_library
cargo test --locked album_creation_persists_an_empty_folder_and_supports_rename
cargo test --locked library_reclassification
cargo test --locked library_move_rolls_back
cargo test --locked older_track_json
cargo test --locked legacy_scan_defaults_library
cargo test --locked album_rename
cargo test --locked changing_a_track_title
cargo test --locked reopen_repairs_the_reported_legacy
cd ../frontend
npm test -- --run
npm run build
cd ..
python tools/control.py test --suite all --report
```

The focused execution on 2026-08-14 reported 93 Rust tests passed, 0 failed, and one opt-in removable-filesystem test ignored. The repository-wide run reported tools 177 passed/21 skipped, frontend 41 passed in six files, schema validation passed, and Tauri structure, Cargo check, and Rust tests passed. TypeScript and the production Vite build also passed. See the [full-suite report](../../../.report/test-report-20260814-144417-suite-all-ok.md), SHA-256 `02f9e5079cd413aa4fe594a0893eed702df5b2b4fb8d5e6c46bb01e04fe99ce8`.

## Verification

Review the native record/tree assertions, TypeScript exact-once grouping, adapter payloads, source-test report, and schema version. Do not promote packaged step 8 from unit or static evidence. A package built from an older commit does not accept this feature.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | No package was built from the physical-folder implementation; real removable-drive rename interaction, collision feedback, modal focus containment/restoration, `Escape`, responsive rendering, and screen-reader behavior were not executed. | High | Product team | Build a clean current package and execute step 8 with retained filesystem snapshots, screenshots, and keyboard/screen-reader notes. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1–7 passed through native/frontend automation and source review; packaged GUI/removable-drive/accessibility step 8 remains `NOT RUN`.
- Residual risks: The physical organization and recovery contract is covered in temporary native workspaces, but the shipped interaction on the user's removable filesystem and keyboard accessibility are not yet accepted.

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
