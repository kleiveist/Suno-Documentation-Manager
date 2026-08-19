<!-- AUTO-GENERATED:backlink START -->
[← Back](usr.md)
<!-- AUTO-GENERATED:backlink END -->
# Getting started with Suno Documentation Manager

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-18 |
| Audience | Suno Documentation Manager users |
| Related ATP | [ATP-0001: Workspace creation and loading](../atp/active/ATP-0001-workspace-creation-and-loading.md); [ATP-0014: Track library organization](../atp/active/ATP-0014-track-library-organization.md); [ATP-0017: Pre-release audio screening](../atp/active/ATP-0017-pre-release-audio-screening.md) |

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

The application asks only for documentation facts. Do not enter credentials or unrelated personal data. A birthday, private telephone number, private email address, Google account, and other private account details are not required global fields. Optional provider credentials are requested only in their dedicated Settings section, remain write-only, and are not copied into a track or certificate.

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

Any workspace folder whose name starts with `.` is treated as hidden management data and is not loaded as an album or track. For example, `.archive/`, `.cache/`, and `.git/` do not appear in the library and are not traversed by workspace scanning. Dot-directories managed inside a normal track folder keep their documented archive and identity purpose.

Selecting a workspace does not authorize access outside that root. If the application reports a symbolic-link, traversal, permission, or collision error, choose a normal contained folder or correct its local permissions; do not bypass the check with a broader filesystem allowlist.

## Complete reusable settings

Open `Einstellungen`. The page is divided into three local sections: `Globale Angaben` for the reusable profile and production defaults (settings 01–04), `Externe Dienste` for the optional timestamp and audio-screening integrations (settings 05–06), and `Globale Datei-Führung` for workspace-wide subscription and archived Terms evidence. The category buttons at the top show only the selected section; they do not save settings or start provider checks.

Setting 04, `Zertifikatssprache`, is also the app language. Select `Deutsch` or `Englisch` and save the settings; navigation, workflow labels, dialogs, and other UI copy switch to the selected language. This affects future finalizations only; existing certificate and track snapshots remain unchanged. The Step 10 bilingual-certificate switch remains an independent choice for adding the second certificate language.

Under `Globale Angaben`, enter the minimal defaults:

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

## Optional ACRCloud audio-screening settings

The local release fingerprint does not need an account or a separate installation: SunoDM uses its bundled Chromaprint engine when you import or replace the authoritative release audio. The external ACRCloud comparison is separate and remains disabled until you choose to configure it.

Under `Einstellungen` → `Externe Dienste` → `Pre-release audio screening`, enter the ACRCloud host, enable the provider, and save the access key and access secret. The fields are write-only: after saving, the UI can report only whether a complete credential pair exists. Use `Provider testen` to check the configured endpoint; this does not upload track audio. Then open Step 09 and choose the external screening action for one specific track. It sends a bounded release-audio sample only after that click, never the local Chromaprint fingerprint. It is optional and cannot block finalization.

## Register subscription evidence

Register each Suno subscription invoice or payment document as its own global-evidence record:

1. Choose the billing cadence shown by that invoice: `Monatlich` or `Jährlich`.
2. Enter the factual first day of the period covered by that invoice. This is the invoice coverage start, not automatically the account-level subscription start date from the reusable settings.
3. Select exactly one supported evidence file in the native file picker: PDF, PNG/JPEG, TXT, or Markdown. Register additional invoices separately; the action does not import a folder or a batch of files.
4. Review the concrete inclusive coverage end that the application calculates from the cadence and start date. A monthly record ends on the day before the next monthly payment date; an annual record ends on the day before the payment date twelve calendar months later. For example, a monthly period beginning `2026-07-01` ends `2026-07-31`, while an annual period beginning `2026-01-01` ends `2026-12-31`.

For a start near the end of a month, the next payment date is first clamped to the last valid day of its target month, then the inclusive end is the preceding day. For example, a monthly period beginning `2026-01-31` ends `2026-02-27`. The same rule handles leap years.

The cadence is a calculation rule for this one document, not permission to extrapolate it indefinitely. A monthly document does not prove later months, and an annual document does not prove a later subscription year. Register the next actual invoice as a separate record, and do not use the calculated period if cancellation, refund, a partial period, or the document itself shows narrower coverage. The source file is preserved.

## Register global Suno terms evidence

Open `Einstellungen` → `Globale Datei-Führung` → `Archivierte Suno-Nutzungsbedingungen`. Enter the document title, provider/source, and retrieval date, then select exactly one local PDF. Add the source URL when known; it is recommended but optional. Effective date, applicable production period, and a factual note are also optional. Leave an unknown optional value undocumented instead of guessing it.

The selected source remains untouched. SunoDM verifies the PDF file signature, registers a hashed copy under `.suno-doc/global-evidence/`, records the original filename, import time, SHA-256 and provenance, and automatically places a linked portable `global_copy` with the same descriptive metadata under `04_LICENSES/` for every new or still editable project. The track copy has its own local Evidence ID and retains `sourceGlobalEvidenceId`; the certificate summary and register use the same local ID. You can complete or correct descriptive metadata in Settings; matching editable track copies are updated and become stale for regeneration. A finalized project is deliberately not changed, so create a revision before assigning newer or corrected Terms.

For a commercially intended track, a Terms file without title, provider/source, or retrieval date does not pass `Evidence & Licenses`. The interface reports that the file exists but its descriptive metadata is incomplete. A verified local Terms file also cannot be marked `Terms evidence not available`: the native save rejects that contradiction, and imported contradictory legacy state remains blocked rather than being rendered into a certificate. A source URL and effective date may remain `NOT DOCUMENTED`. These values describe the archived local version only; SunoDM neither downloads Terms nor determines their validity, enforceability, or legal sufficiency.

For subscription evidence, attach only records whose materialized interval overlaps production or contains final generation. Multiple adjacent selected intervals may jointly provide gap-free production coverage. Before finalization, the application copies each selected file into the track evidence structure, calculates its hash, records `global_copy` provenance and the workspace source record ID, and includes those fields, the exact coverage dates, and the relative path in the manifest. Portability therefore does not depend on rerunning the cadence calculation: the track retains the concrete intervals and remains self-contained if the workspace index is later unavailable.

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

In `02 Source`, choose the applicable source category and rights basis from the guided clickable buttons instead of a dropdown or unrestricted description. Each question permits exactly one active choice. This applies to external audio, own audio, and third-party samples. Answer `Code-based generation?` explicitly. A `No` answer ends that branch. A `Yes` answer first requests the code or text that was actually used, then asks whether its audio output was post-processed, and finally requests the resulting WAV or MP3. If post-processing is `Yes`, select every operation that actually occurred; at least one is required, and `Other post-processing` reveals an editable detail field. `No` creates no operation claim. Supported source formats include Ruby, Python, plain text, Markdown, JavaScript, TypeScript, Rust, shell scripts, and other listed text-based formats. Both managed copies and the factual post-processing answer are included in the generated English project documentation.

In `03 Suno`, record the final generation as separate facts: date, Suno/final-generation ID when known, project URL, optional project/version ID, download/export date, model, and plan at generation. Valid embedded Suno metadata can supply the date and Suno ID with an evidence-derived origin; it never supplies a plan. The model and plan fields offer suggestions but remain freely editable. You can select a listed value such as `v5.5` or `Premier`, or retain an exact historical, custom, or future value such as `v6`; the app does not validate either field against a closed list.

Record only work that occurred. Do not select arrangement, mixing, mastering, or another editing label unless it accurately describes confirmed work on this track.

In `04 Human Work`, keep the Suno Instrumental Mode setting, the Generation Text Field facts, and the actual final-audio result separate. An instrumental track with no vocals may still contain `[Intro]`, `[Drop]`, sound directions, or arrangement instructions and is valid. If the Suno field contains content, select all applicable content types, choose its `human`, `AI`, or `mixed` source, and retain the exact text. `Other` also needs its factual label. The app does not classify bracketed text automatically: a field containing only structure instructions is not automatically Vocal Lyrics or singing. `Instrumental = Yes` together with `Final Audio Contains Vocals = Yes` is a real contradiction and must be corrected before finalization.

Enter the complete Suno style prompt and select only the human-editing steps that actually occurred. Suno structure text is not automatically Human Work; its documented content source controls that statement. If post-export editing is `Yes`, select at least one actual post-export operation; a free-text processing claim is neither shown nor accepted for a new selection. Document generation writes `02_SUNO/Lyrics.md` with a `Vocal Lyrics` or `Suno Structure / Generation Instructions` heading as applicable, plus `02_SUNO/Style.md`. Managed files at the former `03_DOCUMENTATION/Lyrics.md` and `03_DOCUMENTATION/Styles.md` locations are removed during regeneration.

The interface can display guided labels in German, but the corresponding stored values and generated choice statements are English. Exact user-authored facts such as lyrics and the Suno style prompt remain verbatim.

In `05 Artwork`, the visible notice explains that only relevant facts are requested and that the app records your confirmation without making a legal decision. For human artwork, select any number of process chips and freely add or edit process notes. For AI-assisted artwork, select at least one actual human change; `Other human editing` reveals an optional free-text detail. Answer each content check with its independent `Yes` or `No` buttons. Only `Yes` reveals the required factual note for that question; `No` hides and clears it. Upload the final JPG or PNG downloaded from Suno. The final-artwork role is requested exactly once in this step, not again under Release. Three explicit negative artwork answers close their note branches but do not deactivate the separate Audio assessment.

In `06 AI Transparency`, complete Audio and Artwork as separate factual sections. For Audio, first answer `Generative AI used`. `Yes` requires the AI system, the six `Yes`/`No`/`Not documented` indicators for generated/assisted elements and authentic-person/event representations, and an audio-disclosure status. `Disclosure = Yes` requires location and exact text. `Disclosure = No` is a deliberate answer and can retain a factual reason. `Not documented` means the information is missing; for commercial use with generative AI it remains a blocker. The Artwork section continues to show the origin, service, content checks, human changes, policy, and artwork-disclosure result. The app never translates these answers into `AI Act compliant`, `No deepfake`, or another legal conclusion.

Known service values are displayed consistently in new suggestions, for example `ChatGPT / OpenAI`, but every custom or historical free-text value remains unchanged.

In `07 Release`, choose any applicable release-note labels and import the final audio. The managed authoritative copy keeps its real audio extension and is named from the safe track title, for example `01_RELEASE/Neon Universe.wav` or `01_RELEASE/Neon Universe.mp3`. A collision is reported instead of overwritten. The local Chromaprint screening starts against that managed source and Step 07 shows its technical status/source binding; it does not upload audio or make a legal conclusion. Renaming an editable track updates managed release evidence through the native operation; finalized tracks require the existing revision workflow first.

In `09 Integrity`, generate and verify the normal SHA-256 set. The local screening files under `03_DOCUMENTATION/AUDIO_SCREENING/` are included automatically. If you configured ACRCloud and deliberately want the optional comparison, use its explicit action here. A provider response, unavailable provider, authentication failure, or no-match does not change whether the normal integrity operation passes.

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
| `PASS` | Configured documentation requirements for this step were satisfied. It is not a legal or rights decision. |
| `FAIL` | At least one evaluated requirement failed. |
| `BLOCKED` | A prerequisite or deviation prevents completion. |
| `N/A` | The item does not apply and a reason is stored. |
| `NOT VERIFIED` | Imported historical information exists but has not been verified. |

Saving a workflow form reevaluates the requirements and refreshes the rail immediately. Within a questionnaire, distinguish these values:

| Answer | Meaning |
| --- | --- |
| `YES` | You explicitly confirmed the fact. |
| `NO` | You explicitly confirmed that it does not apply or did not occur. |
| `N/A` | The question is logically inapplicable; the application retains the reason. |
| `NOT DOCUMENTED` | Sufficient information is absent. This is not a negative answer. |

Explicit `No` answers count as completed answers where the workflow accepts them; for example, three `No` artwork content checks close their three note branches. An explicit `NOT DOCUMENTED` remains visibly distinct, and the commercial generative-AI disclosure rule can still block completion.

`FAIL`, `BLOCKED`, and `NOT VERIFIED` block finalization. A percentage is a navigation aid; it is not a certificate.

## Reopen or scan a workspace

Use `Workspace auswählen` to reopen the same root. The SQLite index restores mutable working state. Use the scan action to discover unindexed existing track folders.

A scan never changes candidate track files. It adds conservative local index records so found tracks are visible, reports missing and unknown information, and records discovered evidence as `indexed_legacy` and `NOT VERIFIED`. Confirm separately before adopting current workspace profile data or replacing an unmanaged document. See [Legacy track import](../dev/legacy-track-import.md).

Older saved lyrics source/text can appear as legacy compatibility information. Their presence does not answer whether vocals exist or whether the Suno field contains lyrics, structure, sound, or arrangement instructions. Classify those facts explicitly in an editable track or revision; SunoDM does not infer them from `[Intro]`, `[Verse]`, or any other free text.

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
9. Import a disposable final release file and confirm that Step 07 reports a local fingerprint record without exposing its raw fingerprint in the track view.

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
- [Pre-release audio screening](../def/pre-release-audio-screening.md)
- [Legacy track import](../dev/legacy-track-import.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-18 | Added the bundled local fingerprint and explicit optional ACRCloud Settings/Step-09 guidance. | Project team |
| 2026-08-17 | Clarified that verified local Terms evidence cannot coexist with an unavailable claim. | Project team |
| 2026-08-17 | Documented workflow 1.7 Terms metadata, complete Final Suno Generation, separated instrumental/vocal/Suno-field facts, distinct Audio/Artwork AI assessments, provider suggestions, and exact `NO`/`N/A`/`NOT DOCUMENTED` semantics. | Project team |
| 2026-08-16 | Explained conditional code-audio post-processing, free model/plan suggestions, artwork process selections and factual checks, and safe title-based release filenames. | Project team |
| 2026-08-16 | Replaced Source and lyrics-source dropdowns with accessible clickable single-choice buttons. | Project team |
| 2026-08-16 | Documented the required generated WAV/MP3 evidence for code-based generation. | Project team |
| 2026-08-16 | Documented centered final-artwork covers and the initials fallback throughout track presentation. | Project team |
| 2026-08-15 | Explained guided Source and post-export choices, conditional source-code evidence, supported formats, and English document values. | Project team |
| 2026-08-14 | Explained the collapsible top-level and album folder rows and their keyboard operation. | Project team |
| 2026-08-14 | Explained album and single creation, search, reclassification, invariants, and version 0.1 album limits. | Project team |
| 2026-08-14 | Documented per-invoice billing cadence, single-file registration, materialized coverage, and the no-extrapolation rule. | Project team |
| 2026-08-13 | Explained evidence provenance and recoverable indexed-legacy removal. | Project team |
| 2026-08-13 | Added the first-workspace and first-track guide. | Project team |
