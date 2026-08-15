<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Track library organization model

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Product team |
| Last review | 2026-08-16 |
| Audience | Product developers and acceptance reviewers |
| Related ATP | [ATP-0014: Track library album and single organization](../atp/active/ATP-0014-track-library-organization.md) |

## Purpose

This document defines how the track library classifies every indexed track as either an album track or a single and keeps that classification synchronized with the physical workspace folders.

## Scope

### Included

- the permanent `Albums` and `Singles` library sections;
- named album groups and their track membership;
- direct creation and display of empty physical album folders;
- creation, later reclassification, validation, sorting, search, and legacy defaults;
- physical folder creation, movement, renaming, recovery, and rollback;
- bounded, centered track-cover presentation from verified final artwork;
- the persistence and portability boundary; and
- requirements mapped to acceptance evidence.

### Excluded

- album cover art, release dates, catalog numbers, sequencing, or album-level certificates;
- nested disc, edition, playlist, or collection hierarchies;
- deriving an album assignment from a track title, document, or evidence file without a matching physical parent.

## Library hierarchy

The application always presents these two top-level sections, including when a section is empty or a search produces no matching tracks:

```text
Library
├── Albums
│   └── <album title>
│       └── <album tracks>
└── Singles
    └── <single tracks>
```

Every visible indexed track belongs to exactly one section. An album track belongs to exactly one named album folder; a single belongs directly to the permanent `Singles` folder. `Alben` is the UI umbrella for all album folders and is not an additional physical directory.

Opening or creating a workspace ensures the physical `Singles/` folder exists. The `Album anlegen` control in the `Alben` summary creates a named sibling folder immediately, even before a track is assigned. Empty album folders remain visible and survive restart, scanning, and moving their last track elsewhere.

The authoritative workspace layout is:

```text
<workspace>/
├── .suno-doc/
├── <album title>/
│   └── <track title>/
└── Singles/
    └── <track title>/
```

For example:

```text
SunoDocs/
├── Gravity Drift/
│   └── Gravaty/
└── Singles/
    └── Single 1/
```

The hierarchy is presented as a collapsible folder tree. `Albums` and `Singles` are top-level disclosure nodes, and every named album is a nested disclosure node below `Albums`. All nodes start expanded so tracks remain immediately visible. Activating a node header with a pointer, `Enter`, or `Space` collapses or expands only its visible descendants. The implementation uses native HTML `details` and `summary` semantics, retains a visible focus outline, and rotates the disclosure indicator to match the open state.

Collapsing a node is presentation state only. It does not filter, reclassify, persist, move, or otherwise change a track. Search and status changes rebuild the result tree in its expanded state so matching tracks are not hidden by an earlier collapsed view.

## Assignment data

The typed track record contains one top-level library placement:

```json
{
  "library": {
    "section": "album",
    "albumTitle": "Northern Lights"
  }
}
```

`section` is either `album` or `single`. `albumTitle` is required only for `album`. A `single` placement is normalized to contain no album title.

The native layer is authoritative for state-changing validation:

- trim leading and trailing whitespace from an album title;
- require 1 through 200 Unicode characters after trimming;
- reject control characters, path separators, traversal names, and reserved workspace names;
- reject an unknown section value.

Tracks with the same normalized album title appear in one group. Presentation compares the trimmed Unicode-normalized title without case distinctions. It retains one stored spelling for display. Album groups and the tracks inside them are sorted by the German locale with numeric ordering. This grouping does not rewrite the stored spelling of another track.

## Creation, movement, and renaming

The `Alben` section header exposes `Album anlegen`. It validates the title through the same native boundary as track assignment, creates exactly one empty physical album folder, rejects case-insensitive duplicates and collisions, and returns the refreshed folder list. New-track creation asks the user to choose `Single` or `Album track`. Choosing an album reveals the required album-title field and offers every physical album title as an input suggestion. The native create operation creates the physical parent and track folders before storing the normalized assignment and relative path. Folder names retain the trimmed visible album and track titles. The native layer rejects reserved, traversal, separator, control-character, and collision cases before moving data.

The current-track header exposes a separate library-assignment action. It can move a track:

- from `Singles` to a named album;
- from an album to `Singles`; or
- from one album name to another.

The assignment action moves the complete track root to its new parent. It remains available for a `FINALIZED` track because paths inside the track root do not change. It must preserve all of these values:

- track ID, title, creation time, and update time; the workspace-relative path changes to the new physical location;
- lifecycle status, workflow ID and version, step state, profile snapshot, and track fields;
- evidence and blocking deviations;
- generated-document, integrity, and certificate state; and
- every file and directory below the portable track root.

Reclassification therefore does not invalidate a certificate or create a revision. The application checks the destination first, never merges or overwrites an existing folder, persists the new path after the move, and moves the folder back if the database write fails. Empty album folders remain available for later tracks; an album folder created only as part of a failed move is removed during rollback. The permanent `Singles` parent always remains.

Each album header exposes `Umbenennen`. Renaming an album moves the complete album directory once and updates every contained track path and album title in one SQLite transaction. A destination collision stops the operation without overwriting either album. Changing a non-finalized track title through its track fields also renames that track's leaf directory. Normal finalized-track editing rules still apply to a title change because the title is certificate content.

## Track cover presentation

A verified `final_artwork` evidence item identifies the visual cover for its track. Dashboard rows, the attention card, album/single library rows, and the current-track header use the same centered square preview. This is track-level presentation only; named album folders still have no separate album artwork.

The native layer validates the contained managed PNG/JPG and creates a 192 × 192 PNG preview with a centered crop outside the UI thread. It bounds source bytes and decoded pixel count before processing. The frontend requests only tracks whose summary identifies a current verified final-artwork record, loads at most three previews concurrently, caches the result for the open workspace, and rejects a late result when the evidence ID or workspace changed. The preview is not persisted, hashed, or treated as new evidence. Missing, removed, unverified, changed, unsupported, or unsafe artwork retains the deterministic initials tile without blocking track navigation.

## Search and status filters

Search operates within the hierarchy. It matches track title, track-relative path, and album title without case distinctions. An album-title match includes all tracks in that album that also pass the active status filter. A track-title or path match includes only the matching tracks while preserving their album group.

The status filter is applied before presentation:

- `All` includes every indexed status;
- `Open` includes `DRAFT` and `ACTIVE`;
- `Ready` includes `READY`; and
- `Finalized` includes `FINALIZED`.

The two top-level sections remain visible when their filtered result is empty.

## Persistence and recovery boundary

The typed library placement and workspace-relative track path are stored in `.suno-doc/workspace.sqlite`. Existing track records whose JSON predates the placement field deserialize as `single`. The physical parent is authoritative when a managed track is found below `Singles/` or a named album folder.

This additive JSON field does not change the relational SQLite layout, so the schema remains version `2` and no relational migration runs. Loading and subsequently saving an older record materializes the default in its JSON.

New managed tracks contain `.summary/track.json`, which stores only the stable track ID. `.summary/` is already excluded from the track integrity set and certificate file set. The marker lets reopening or scanning reconnect a managed track after its track folder or album parent was renamed outside the application. It does not alter evidence, generated documentation, hashes, or certificate bytes.

Workspace scanning recognizes all three supported layouts: managed singles below `Singles/`, managed or historical tracks below a named album folder, and historical direct-child track roots from versions predating this hierarchy. An empty album is listed as a library folder but never indexed as a track. A historical scan remains read-only for the candidate folder and does not add the identity marker. When a stale legacy index path is missing and exactly one unclaimed candidate exists in its assigned album folder, reopening repairs the database path conservatively. Ambiguous candidates remain unresolved instead of being guessed.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-LIB-001` | The library always renders `Albums`, every physical named album including empty folders, and `Singles` as nested collapsible nodes, places each filtered track exactly once below its parent, groups valid album assignments by normalized title, searches track and album text within the hierarchy, and uses a bounded centered verified final-artwork preview with an initials fallback for each track. | [ATP-0014](../atp/active/ATP-0014-track-library-organization.md), [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) |
| `REQ-LIB-002` | Workspace opening creates the permanent `Singles/` folder. Direct album creation, track creation, and reclassification validate and persist their physical paths; moves preserve every track-root byte and protected track state and roll back on database failure. | [ATP-0014](../atp/active/ATP-0014-track-library-organization.md) |
| `REQ-LIB-003` | Album and track folders can be renamed without overwriting collisions; every affected SQLite path is updated, and managed external renames are recovered by stable identity. | [ATP-0014](../atp/active/ATP-0014-track-library-organization.md) |

## Verification

Run the native and frontend suites from the repository root:

```sh
python tools/control.py test --suite tauri
python tools/control.py test --suite frontend
python tools/control.py test --suite all --report
```

The focused native tests cover physical creation, validation, create and reopen, older JSON, nested and direct legacy scan, finalized-track movement, album rename, track-folder rename, external rename recovery, and stale legacy-path repair. The focused frontend tests cover hierarchy, exact-once grouping, sorting, search, status filtering, validation, demo path changes, and typed command mapping. Packaged interaction remains separately identified in ATP-0014.

## Risks and limitations

- Album identity remains name-based and filesystem-backed; an empty album has no separate database record or release metadata.
- Two visually similar Unicode titles may remain separate if their normalized comparison keys differ.
- A legacy track without a managed identity marker may require an unambiguous folder location for automatic stale-path recovery.
- Version 0.1 does not provide manual track order within an album.

## Related documents

- [Getting started](../usr/getting-started.md)
- [Product architecture](product-architecture.md)
- [Persistence and recovery](persistence.md)
- [Track documentation model](track-documentation-model.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-16 | Added centered, bounded final-artwork previews and stale-result/fallback behavior for track covers. | Product team |
| 2026-08-14 | Made album/single placement physical, added safe folder renaming, stable managed identity recovery, and stale legacy-path repair. | Product team |
| 2026-08-14 | Defined the nested, collapsible folder presentation and its non-persistent disclosure state. | Product team |
| 2026-08-14 | Defined the album and single library hierarchy, invariants, persistence boundary, and acceptance requirements. | Product team |
