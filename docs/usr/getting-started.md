<!-- AUTO-GENERATED:backlink START -->
[← Back](usr.md)
<!-- AUTO-GENERATED:backlink END -->
# Getting started with Suno Documentation Manager

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-15 |
| Audience | Suno Documentation Manager users |
| Related ATP | [ATP-0001: Workspace creation and loading](../atp/active/ATP-0001-workspace-creation-and-loading.md); [ATP-0014: Track library organization](../atp/active/ATP-0014-track-library-organization.md) |

## Purpose

This guide explains how to open the local desktop application, create or select a workspace, organize its track library, enter reusable settings, and start documenting a track without exposing private data or overwriting existing evidence.

## Scope

### Included

- first launch, appearance choice, and workspace selection;
- minimal global settings;
- album and single library organization, new-track creation, and the ten-step navigation;
- evidence import and missing-item feedback; and
- reopening or scanning an existing workspace.

### Excluded

- building release packages;
- accepting a legacy document as application-managed content;
- detailed finalization and revision handling; and
- legal advice about a track, license, or artwork.

## Before you begin

Choose a local folder that you can read and write and that can contain one child folder per track. Keep an independent backup appropriate for your music projects. Normal product use does not require an account, backend service, or internet connection.

The application asks only for documentation facts. Do not enter credentials or unrelated personal data. A birthday, private telephone number, private email address, Google account, and other private account details are not required global fields.

## Launch the application

Open the installed desktop application. Contributors can launch a complete development instance from the repository root with:

```sh
python tools/control.py tauri run --foreground
```

Use the Tauri window for workspace and evidence operations. A standalone browser preview cannot perform the native file, SQLite, hashing, artwork, or certificate operations.

## Choose the appearance

The application follows the operating-system light or dark preference on first launch. Use the sun or moon button in the top bar to switch between light and dark mode at any time. The same control is available on the workspace-selection screen.

An explicit choice is stored locally on this device and restored before the interface is drawn on later launches. It affects only the application appearance: workspace files, evidence, generated documents, and certificates are unchanged and the setting is never synchronized to a service.

## Create or open a workspace

On first launch the application shows that no workspace is open and offers two native actions:

- `Workspace auswählen` selects an existing local music-project folder.
- `Neuen Workspace anlegen` creates a new local folder and initializes it as a workspace.

The application canonicalizes the selected root before it creates or opens `<workspace>/.suno-doc/`. This hidden management folder holds the local SQLite index and reusable workspace evidence. It is not part of any track evidence set.

Selecting a workspace does not authorize access outside that root. If the application reports a symbolic-link, traversal, permission, or collision error, choose a normal contained folder or correct its local permissions; do not bypass the check with a broader filesystem allowlist.

## Complete reusable settings

Open `Einstellungen` and enter the minimal defaults:

| Setting | What to enter |
| --- | --- |
| Artist name | The artist label that should appear in track snapshots |
| Suno profile name | The relevant public profile name |
| Suno handle | The relevant public handle |
| Suno plan | The default plan; confirm the actual plan for each track |
| Suno subscription start date | The factual subscription start date |
| Default commercial use intended | The normal intent; confirm it per track |
| Default AI image service | The service used when artwork is AI-generated or AI-assisted |
| AI artwork transparency policy | `Always add visible AI disclosure`, `Decide per artwork`, or `No automatic visible disclosure` |

The project default transparency policy is `Always add visible AI disclosure`. This is a project transparency choice, not a statement that a particular watermark is universally required by law. The default disclosure text is `AI-assisted` and can be configured.

Saving global settings updates all open tracks and marks their generated documents stale, so artist name, Suno profile, handle, plan, and policy do not remain `Not documented`. Opening an older workspace also assigns its already saved global values to every non-finalized track. Fulfilled legacy steps then leave `Nicht verifiziert` automatically. Finalized and superseded tracks are not rewritten. Generated documents contain the profile snapshot actually assigned to that track.

## Register subscription evidence

Register each Suno subscription invoice or payment document as its own global-evidence record:

1. Choose the billing cadence shown by that invoice: `Monatlich` or `Jährlich`.
2. Enter the factual first day of the period covered by that invoice. This is the invoice coverage start, not automatically the account-level subscription start date from the reusable settings.
3. Select exactly one supported evidence file in the native file picker: PDF, PNG/JPEG, TXT, or Markdown. Register additional invoices separately; the action does not import a folder or a batch of files.
4. Review the concrete inclusive coverage end that the application calculates from the cadence and start date. A monthly record ends on the day before the next monthly payment date; an annual record ends on the day before the payment date twelve calendar months later. For example, a monthly period beginning `2026-07-01` ends `2026-07-31`, while an annual period beginning `2026-01-01` ends `2026-12-31`.

For a start near the end of a month, the next payment date is first clamped to the last valid day of its target month, then the inclusive end is the preceding day. For example, a monthly period beginning `2026-01-31` ends `2026-02-27`. The same rule handles leap years.

The cadence is a calculation rule for this one document, not permission to extrapolate it indefinitely. A monthly document does not prove later months, and an annual document does not prove a later subscription year. Register the next actual invoice as a separate record, and do not use the calculated period if cancellation, refund, a partial period, or the document itself shows narrower coverage. The source file is preserved.

When documenting a track, select only evidence whose materialized start and end dates actually cover its production period. Before finalization, the application copies the selected file into the track evidence structure, calculates its hash, records `global_copy` provenance and the workspace source record ID, and includes those fields, the exact coverage dates, and the relative path in the manifest. Portability therefore does not depend on rerunning the cadence calculation: the track retains the concrete interval and remains self-contained if the workspace index is later unavailable.

## Create a track

Open `Tracks` and choose the new-track action. Enter the track title and production start, then select one library placement:

- `Single` creates the track below the permanent physical `Singles/` folder.
- `Album-Track` reveals a required album-title field. Select an existing suggestion or enter a new album title to create the track below that physical album folder.

Album titles are trimmed, limited to 200 characters, and cannot contain control characters, path separators, traversal names, or reserved workspace names. Titles that differ only by case are presented as the same album group. The visible trimmed track title is also used as its folder name. If a file or folder already occupies the destination, the application reports a collision and does not overwrite it.

You can also select `Album anlegen` directly in the `Alben` folder header. Entering `Gravity Drift` immediately creates `<workspace>/Gravity Drift/` and displays the empty, collapsible album. The permanent `<workspace>/Singles/` folder is created automatically when the workspace opens. Assigning tracks later produces `Gravity Drift/<track title>/` or `Singles/<track title>/`.

The resulting structure is visible without the application:

```text
SunoDocs/
├── Gravity Drift/
│   └── Gravaty/
└── Singles/
    └── Single 1/
```

A new track begins as `DRAFT`. The application creates the standard directories but no empty WAV, MP3, MP4, image, PDF, or ZIP placeholders. Only imported real evidence fills evidence roles.

## Organize existing tracks

The `Tracks` page always shows `Alben` and `Singles` as folder-like rows. Select either row to collapse or expand the whole section. Each named album below `Alben` is another collapsible row; its assigned tracks remain nested underneath it. The rows start expanded and support pointer clicks, `Enter`, and `Space`. The library search matches a track title, track path, or album title. Status buttons filter tracks inside both sections; they do not remove the two top-level rows.

After a verified final JPG or PNG has been imported for a track, its centered square artwork preview replaces the initials tile in dashboard lists, the attention card, the album/single library rows, and the current-track header. Until then, or if the managed image cannot be decoded safely, the initials remain visible. The preview is a small local derivative used only for display; the managed final-artwork file remains the authoritative evidence and is not changed.

To change an existing assignment:

1. Open the track.
2. Select its `Single` or `Album · <album title>` library chip in the track header.
3. Choose `Single` or `Album-Track` and, for an album, enter or select its title.
4. Save the assignment.

Saving the assignment moves the complete track folder to the selected physical parent. It does not change files inside the track folder, update the track timestamp, change workflow or finalization status, regenerate documents, recalculate hashes, or invalidate a certificate. It is therefore also available for a finalized track. A destination collision stops the operation without overwriting data; a database failure moves the folder back.

To rename an album folder, including an empty album, select `Umbenennen` in its album header and enter the new folder name. The application moves the album once and updates all contained track paths together. Changing an editable track title also renames that track's folder. A finalized track title still requires the normal revision workflow because the title is certificate content.

An album is a named physical folder for indexed tracks, not an independent release object. Empty albums are supported and remain visible until they are renamed or used; they do not contain a separate album database record. Version 0.1 does not store album artwork, release metadata, track order, or a separate album certificate. Existing direct-child historical track folders remain supported. A scan recognizes tracks below `Singles/` and named album folders without treating an empty album as a track.

## Follow the documentation steps

Use the current-track view to work through:

1. `01 Track`
2. `02 Source`
3. `03 Suno`
4. `04 Human Work`
5. `05 Artwork`
6. `06 AI Transparency`
7. `07 Release`
8. `08 Evidence & Licenses`
9. `09 Integrity`
10. `10 Finalize`

The application displays one task-oriented set of questions at a time. A negative controlling answer closes its branch. For example, answering `No` to external audio upload means source, ownership, license, and uploaded-file details for external audio are not requested. Answering `Yes` makes those details applicable.

In `02 Source`, choose the applicable source category and rights basis from the guided clickable buttons instead of a dropdown or unrestricted description. Each question permits exactly one active choice. This applies to external audio, own audio, and third-party samples. Answer `Code-based generation?` explicitly. A `No` answer ends that branch. A `Yes` answer opens two evidence controls: one for the code or text that was actually used and one for the resulting WAV or MP3. Supported source formats include Ruby, Python, plain text, Markdown, JavaScript, TypeScript, Rust, shell scripts, and other listed text-based formats. Both managed copies are stored below `02_SUNO/` and their paths are included in the generated English project documentation.

Record only work that occurred. Do not select arrangement, mixing, mastering, or another editing label unless it accurately describes confirmed work on this track.

In `04 Human Work`, choose exactly one lyrics source from the clickable single-choice buttons, record the exact lyrics text used for every non-instrumental source, and enter the complete Suno style prompt. Select all actually performed human-editing steps from the guided multi-choice buttons. If post-export editing is `Yes`, select at least one actual post-export operation; a free-text processing claim is neither shown nor accepted for a new selection. Document generation writes `02_SUNO/Lyrics.md` and `02_SUNO/Style.md`; managed files at the former `03_DOCUMENTATION/Lyrics.md` and `03_DOCUMENTATION/Styles.md` locations are removed during regeneration.

The interface can display guided labels in German, but the corresponding stored values and generated choice statements are English. Exact user-authored facts such as lyrics and the Suno style prompt remain verbatim.

In `05 Artwork`, answer the three content checks and upload the final JPG or PNG downloaded from Suno. The final-artwork role is requested exactly once in this step, not again under Release. If a file already occupies the role but does not satisfy the current rule, the open requirement uses the safe replacement action and archives the previous managed bytes. For AI artwork, three explicit `No` answers deactivate `06 AI Transparency`; otherwise the configured disclosure policy applies. In `07 Release`, choose any applicable release-note labels and import the final audio files.

## Import evidence

Use the native evidence picker from the relevant step:

1. Select the original local file.
2. Choose or confirm its evidence role.
3. Review the proposed destination and any detected conflict.
4. Confirm the copy.
5. Check that the application reports the copied file, size, SHA-256 digest, and updated workflow state.

Every evidence control states the accepted file types. After a file is present, select the large checked area to open its in-app preview. PNG, JPEG, and WebP images are shown as images; small TXT, Markdown, JSON, and supported source-code files are shown as text. Large images, large text files, archives, audio, video, and PDF files show safe metadata and an explanation instead of being loaded wholesale. In particular, ZIP files are never unpacked for preview.

The separate upload button on the right replaces the selected evidence record explicitly. The replacement keeps the record identity, writes and hashes the new managed copy, and moves the previous managed bytes below `.archive/evidence-replacements/`. If the evidence-record update fails, the filesystem change is rolled back. A normal import to an already registered relative path produces a controlled message directing the user to this replacement action; it does not surface a raw SQLite `UNIQUE` error.

The application copies evidence; it does not move or delete the source. A normal track import receives `managed_copy` provenance. It never silently replaces an existing destination. If a same-name file already exists without a matching indexed record, resolve the conflict explicitly instead of assuming that the files are identical.

Large evidence such as a project ZIP is copied and SHA-256-hashed in one streamed background operation, so the webview remains responsive and the source is read only once. Normal track and library loading checks the path, regular-file status, and stored size without repeatedly hashing evidence larger than 64 MiB. The explicit evidence verification, hash calculation, hash verification, and finalization paths still perform the required full cryptographic reads.

The provenance label in the evidence list distinguishes a managed import from a copied global record, a locally generated disclosure, or a file discovered in a historical folder. A role describes the file's purpose; it is not proof of how the file was created.

## Read the dashboard

The track dashboard emphasizes progress and concrete missing items. Step labels use these meanings:

| Status | Meaning |
| --- | --- |
| `NOT RUN` | The step has no valid result yet. |
| `PASS` | All applicable mandatory requirements in the step pass. |
| `FAIL` | At least one evaluated requirement failed. |
| `BLOCKED` | A prerequisite or deviation prevents completion. |
| `N/A` | The item does not apply and a reason is stored. |
| `NOT VERIFIED` | Imported historical information exists but has not been verified. |

Saving a workflow form reevaluates the requirements and refreshes the rail immediately. Explicit `No` answers count as completed answers; for example, three `No` answers in the artwork content check close all three note branches and allow the artwork requirement to pass once its other applicable fields and evidence are complete.

`FAIL`, `BLOCKED`, and `NOT VERIFIED` block finalization. A percentage is a navigation aid; it is not a certificate.

## Reopen or scan a workspace

Use `Workspace auswählen` to reopen the same root. The SQLite index restores mutable working state. Use the scan action to discover unindexed existing track folders.

A scan never changes candidate track files. It adds conservative local index records so found tracks are visible, reports missing and unknown information, and records discovered evidence as `indexed_legacy` and `NOT VERIFIED`. Confirm separately before adopting current workspace profile data or replacing an unmanaged document. See [Legacy track import](../dev/legacy-track-import.md).

If you explicitly remove indexed legacy evidence after review, the application moves a present file to `.archive/removals/<removal-id>/`, writes a `removal.json` audit record, and removes it from the index. It does not permanently delete the historical bytes, and a later scan does not re-add the archived path. There is no automatic restore action in version 0.1, so preserve the removal directory if you may need manual recovery.

## Verification

For a manual smoke check, use a temporary workspace containing no private or production data:

1. Create the workspace and confirm `.suno-doc/` appears only inside it.
2. Save the minimal global settings and close the application.
3. Reopen the workspace and confirm the settings are restored.
4. Create a track and confirm the standard directories exist with no fake evidence files.
5. Create one album track and one single, then confirm they appear exactly once in the corresponding library hierarchy.
6. Reassign the album track to `Singles` and confirm its folder path and documentation state do not change.
7. Import a disposable evidence file and confirm the source still exists.
8. Attempt the same destination again and confirm that the application reports a collision.

Executed results and outstanding manual checks are recorded in [ATP-0001](../atp/active/ATP-0001-workspace-creation-and-loading.md), [ATP-0002](../atp/active/ATP-0002-track-creation.md), [ATP-0014](../atp/active/ATP-0014-track-library-organization.md), and the relevant evidence ATP.

## Troubleshooting

- If no native dialog opens, confirm that you launched the Tauri application rather than only a browser preview.
- If a workspace cannot be opened, verify local read/write permissions and that it is a directory.
- If a path is rejected, remove traversal components or an escaping symbolic link; do not widen application permissions.
- If an evidence import collides, compare the files and choose an explicit resolution. The application preserves both the source and existing destination.
- If removable storage reports that a file operation is not permitted, first verify that the selected filesystem is mounted read/write and retry with the current build. The Linux/exFAT regression check is recorded in ATP-0012; this does not establish support for every removable filesystem or operating system.
- If a historical track remains `NOT VERIFIED`, supply factual information or evidence; do not invent a value solely to increase progress.
- Do not place a workspace where an untrusted process running as the same operating-system user can modify it concurrently. Version 0.1 rejects observed symbolic links but does not claim race-free protection against a path component swapped after validation.

## Related documents

- [Finalizing a track](finalizing-a-track.md)
- [Track documentation model](../def/track-documentation-model.md)
- [Track library organization model](../def/track-library-model.md)
- [Workflow model](../def/workflow-model.md)
- [Persistence and recovery](../def/persistence.md)
- [Legacy track import](../dev/legacy-track-import.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-16 | Replaced Source and lyrics-source dropdowns with accessible clickable single-choice buttons. | Project team |
| 2026-08-16 | Documented the required generated WAV/MP3 evidence for code-based generation. | Project team |
| 2026-08-16 | Documented centered final-artwork covers and the initials fallback throughout track presentation. | Project team |
| 2026-08-15 | Explained guided Source and post-export choices, conditional source-code evidence, supported formats, and English document values. | Project team |
| 2026-08-14 | Explained the collapsible top-level and album folder rows and their keyboard operation. | Project team |
| 2026-08-14 | Explained album and single creation, search, reclassification, invariants, and version 0.1 album limits. | Project team |
| 2026-08-14 | Documented per-invoice billing cadence, single-file registration, materialized coverage, and the no-extrapolation rule. | Project team |
| 2026-08-13 | Explained evidence provenance and recoverable indexed-legacy removal. | Project team |
| 2026-08-13 | Added the first-workspace and first-track guide. | Project team |
