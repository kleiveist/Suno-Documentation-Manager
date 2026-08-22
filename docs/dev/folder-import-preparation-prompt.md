<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Codex CLI prompt for folder-import preparation

| Field | Value |
| --- | --- |
| Status | Draft |
| Owner | Project team |
| Last review | 2026-08-22 |
| Audience | Maintainers preparing source folders for import review |
| Related ATP | [ATP-0003: Legacy track import](../atp/active/ATP-0003-legacy-track-import.md) |

## Purpose

This work-in-progress note provides a reusable Codex CLI prompt for preparing a separate copy of a music folder for the application's **Import folder** action. It keeps the source folder intact and produces an explicit readiness report before a person imports anything.

## Scope

### Included

- a safe copy-based preparation workflow for a single track or an album;
- import-recognized names, extensions, direct-file layout, and duplicate detection;
- a consistency report with SHA-256 values; and
- clear handling of files the bulk folder importer cannot take over.

### Excluded

- changing the original source folder;
- altering audio, images, archives, timestamps, or embedded metadata;
- inventing factual workflow answers or treating a filename as provenance; and
- importing, finalizing, or verifying a track inside the application.

## Important importer boundaries

The current folder importer deliberately copies only files that have one unique, type-valid automatic role. It scans direct regular files only.

- A **single** import is one source folder whose direct files belong to one track.
- An **album** import needs at least two direct child folders, each with at least one valid direct MP3, MP4/M4V, or WAV file. Root-level files are reported as unassigned and are not copied.
- Nested source files, symbolic links, duplicate role candidates, generic PNG files, final artwork, PDFs, licenses, and unsupported formats are not bulk-imported by this feature. They must remain listed for a later explicit evidence import, or be handled through the separate legacy scan when they already belong to a historical SunoDM track tree.
- A valid Suno Studio WAV can additionally be recognized as a Suno final export from its embedded structured metadata. Its name alone must not claim that role.

The preparation copy is therefore *ready for the supported automatic part of the import*, not evidence that every file was imported or verified. The application creates incomplete editable tracks; the remaining workflow facts and evidence still need review.

## Codex CLI prompt

Replace the three placeholders, start Codex CLI with access to both folders, and paste the following prompt. The prepared folder must not be inside the source folder or the SunoDM workspace.

~~~text
Prepare a safe, import-ready copy of a music folder for Suno Documentation Manager's "Import folder" action.

Inputs
- Original source folder (read-only): <SOURCE_FOLDER>
- New prepared-copy destination (may be created or replaced only after you show its exact resolved path and confirm it is separate from the source and SunoDM workspace): <PREPARED_FOLDER>
- SunoDM workspace, used only to ensure the prepared copy is outside it: <WORKSPACE_FOLDER>

Goal
Create <PREPARED_FOLDER> as a byte-preserving preparation copy and make the supported files unambiguous for the folder importer. Do not import anything into SunoDM. Do not change, delete, rename, move, or overwrite files in <SOURCE_FOLDER>. Do not modify file bytes, audio metadata, image metadata, timestamps, archives, or documents. Do not create symbolic links. Do not invent missing facts, dates, licenses, authorship, or verification results.

First inspect the source recursively and report:
1. whether it is a single-track candidate or an album candidate;
2. every file, its relative path, size, SHA-256, extension, and detected file type where practical;
3. duplicate or ambiguous candidates, nested files, symlinks, broken links, unsupported files, and root-level album files; and
4. the exact source and destination paths after canonicalization.

Stop and ask for direction instead of modifying anything if:
- the destination resolves inside the source or workspace, or the source resolves inside the destination;
- a symbolic link, unreadable file, path traversal concern, or a destination collision makes the copy unsafe;
- the album/single interpretation is ambiguous; or
- preserving every file would require a conversion, metadata rewrite, or an unsupported automatic role.

After the safety checks, create a byte-for-byte copy at <PREPARED_FOLDER>. Preserve the original folder hierarchy in an audit area outside the importer-facing track roots if needed, but do not represent audit-only files as importable. The importer scans direct regular files only:
- Single: place each selected supported file directly in <PREPARED_FOLDER>.
- Album: place each selected supported file directly in its direct track-child folder. An album needs at least two such child folders with valid direct audio/video.

Use only these automatic recognition rules. Rename files in the prepared copy only when this makes a role unambiguous and the content already truthfully has that role. Never use a filename to claim a fact that cannot be established from the source.
- one valid .mp3 -> release_mp3;
- one valid .mp4 or .m4v -> release_mp4;
- one valid .wav -> release_wav; retain embedded metadata unchanged. It can also become suno_final_export only when the importer can detect genuine Suno Studio metadata in the WAV;
- one .zip whose tokenized filename contains stems -> suno_project_zip (for example, Track_STEMS.zip);
- one PNG/JPG/JPEG/WEBP whose tokenized name contains screenshot or bildschirmfoto -> suno_screenshot;
- one JPG or JPEG without a screenshot name -> artwork_suno_original;
- one PNG/JPG/JPEG named with tokenized AI and ORIGINAL -> ai_artwork_original;
- one PNG/JPG/JPEG named with tokenized AI and EDITED or EDIT -> ai_artwork_edited;
- one non-AI PNG/JPG/JPEG named with tokenized EDITED or EDIT -> human_edited_artwork;
- one source file with .rb, .py, .js, or .ts -> source_code_file;
- one UTF-8 .txt or .md named with tokenized lyrics, lyric, or songtext -> lyrics;
- one UTF-8 .txt or .md named with tokenized style, stil, prompt, or sunostyle -> style.

For every automatic role, retain at most one candidate per track. Do not solve duplicates by guessing or deleting. Put unselected originals in the audit inventory and report the required manual decision. Do not attempt to bulk-import final artwork, PDFs, licenses, invoices, generic PNG images, external timestamps, or arbitrary Other files; report them as manual follow-up items.

Write a UTF-8 IMPORT_READINESS.md next to (not inside) <PREPARED_FOLDER>. Include:
- canonical source, prepared-copy, and workspace paths;
- detected import kind and proposed track titles;
- a table of selected files with prepared relative path, intended role, source relative path, size, SHA-256, and why the role is unambiguous;
- all unassigned/manual-follow-up files with the exact reason;
- duplicate, type-validation, and layout findings;
- a statement that hashes prove byte identity only and do not verify provenance, rights, authorship, or workflow facts; and
- the final user action: open a SunoDM workspace outside the prepared copy, choose "Import folder", select <PREPARED_FOLDER>, inspect the proposal, and import only if the proposal matches this report.

Finish with a concise summary and do not perform any further action after writing the report.
~~~

## Verification

Before using the prepared copy, a reviewer checks that:

1. the report lists the source and prepared-copy paths as separate locations;
2. every selected file has matching source and prepared-copy SHA-256 values;
3. each selected file is a direct regular file in the applicable single or track-child directory;
4. each automatic role has no more than one candidate per track; and
5. every excluded file has a visible follow-up decision rather than silently disappearing.

In the application, use the preview shown by **Import folder** as the final importability check. The preview must agree with the report's import kind, number of tracks, selected files, ambiguities, and unassigned files. A changed folder must be scanned again before import.

The implementation behavior is covered by the folder-import unit tests and application tests:

~~~sh
python tools/control.py test --suite tauri
~~~

## Risks and limitations

- The bulk importer does not provide complete recursive archival. The audit report prevents this limitation from being mistaken for complete content migration.
- File names are bounded hints. They do not prove provenance or legal status.
- The actual import rejects a destination inside the selected source folder. Keep the prepared copy and the workspace separate.
- Imported tracks deliberately remain incomplete until facts, additional evidence, hashes, and the workflow review are completed in SunoDM.

## Related documents

- [Legacy track import and managed-document adoption](legacy-track-import.md)
- [Track documentation model](../def/track-documentation-model.md)
- [Getting started with Suno Documentation Manager](../usr/getting-started.md)
- [Documentation Standard](../README.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-22 | Added a copy-safe Codex CLI prompt and importer-consistency checklist for WIP use. | Project team |

