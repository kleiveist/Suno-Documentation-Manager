<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Track library organization model

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Product team |
| Last review | 2026-08-14 |
| Audience | Product developers and acceptance reviewers |
| Related ATP | [ATP-0014: Track library album and single organization](../atp/active/ATP-0014-track-library-organization.md) |

## Purpose

This document defines how the track library classifies every indexed track as either an album track or a single without changing the portable track snapshot.

## Scope

### Included

- the permanent `Albums` and `Singles` library sections;
- named album groups and their track membership;
- creation, later reclassification, validation, sorting, search, and legacy defaults;
- the persistence and portability boundary; and
- requirements mapped to acceptance evidence.

### Excluded

- album cover art, release dates, catalog numbers, sequencing, or album-level certificates;
- empty albums without tracks;
- nested disc, edition, playlist, or collection hierarchies;
- moving track folders into physical album or single directories; and
- deriving an album assignment from a track title, folder name, document, or evidence file.

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

An album is a name-based library group in version 0.1. It is not a separate release record or filesystem object. Every visible indexed track belongs to exactly one section. An album track belongs to exactly one album group; a single belongs directly to `Singles`.

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
- reject control characters; and
- reject an unknown section value.

Tracks with the same normalized album title appear in one group. Presentation compares the trimmed Unicode-normalized title without case distinctions. It retains one stored spelling for display. Album groups and the tracks inside them are sorted by the German locale with numeric ordering. This grouping does not rewrite the stored spelling of another track.

## Creation and reclassification

New-track creation asks the user to choose `Single` or `Album track`. Choosing an album reveals the required album-title field and offers existing album titles as input suggestions. The native create operation stores the normalized assignment together with the new track record.

The current-track header exposes a separate library-assignment action. It can move a track:

- from `Singles` to a named album;
- from an album to `Singles`; or
- from one album name to another.

This action is organizational metadata only. It remains available for a `FINALIZED` track and must preserve all of these values:

- track ID, title, relative path, creation time, and update time;
- lifecycle status, workflow ID and version, step state, profile snapshot, and track fields;
- evidence and blocking deviations;
- generated-document, integrity, and certificate state; and
- every file and directory below the portable track root.

Reclassification therefore does not invalidate a certificate or create a revision. Content-changing commands continue to use their existing lifecycle rules.

## Search and status filters

Search operates within the hierarchy. It matches track title, track-relative path, and album title without case distinctions. An album-title match includes all tracks in that album that also pass the active status filter. A track-title or path match includes only the matching tracks while preserving their album group.

The status filter is applied before presentation:

- `All` includes every indexed status;
- `Open` includes `DRAFT` and `ACTIVE`;
- `Ready` includes `READY`; and
- `Finalized` includes `FINALIZED`.

The two top-level sections remain visible when their filtered result is empty.

## Persistence and recovery boundary

The library placement is workspace-index metadata stored as part of the typed track JSON in `.suno-doc/workspace.sqlite`. Existing track records whose JSON predates this field deserialize as `single`. A workspace scan also assigns `single` to a newly indexed historical track because the filesystem does not prove album membership.

This additive JSON field does not change the relational SQLite layout, so the schema remains version `2` and no relational migration runs. Loading and subsequently saving an older record materializes the default in its JSON.

The placement is deliberately excluded from the portable track folder, managed documents, hash list, evidence manifest, and certificate. If the workspace index is lost and only a track folder remains, scanning can recover the track as a single but cannot recover its prior album assignment. A complete workspace backup must therefore include `.suno-doc/` when preserving library organization matters.

Physical `Albums/` and `Singles/` directories are not created. Workspace scanning treats eligible direct child directories as track roots, so inserting another physical layer would change the established track-discovery contract and portable paths.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-LIB-001` | The library always renders `Albums` and `Singles`, groups valid album assignments by normalized title, places each filtered track exactly once, and searches track and album text within the hierarchy. | [ATP-0014](../atp/active/ATP-0014-track-library-organization.md) |
| `REQ-LIB-002` | Create and reclassification validate and persist the assignment, old and scanned records default to `single`, and reclassifying any lifecycle status changes no portable track content or protected track state. | [ATP-0014](../atp/active/ATP-0014-track-library-organization.md) |

## Verification

Run the native and frontend suites from the repository root:

```sh
python tools/control.py test --suite tauri
python tools/control.py test --suite frontend
python tools/control.py test --suite all --report
```

The focused native tests cover validation, create and reopen, older JSON, legacy scan, and finalized-track reclassification. The focused frontend tests cover hierarchy, exact-once grouping, sorting, search, status filtering, normalization, and typed command mapping. Packaged interaction remains separately identified in ATP-0014.

## Risks and limitations

- Album identity is name-based. Renaming an album means reclassifying its tracks; there is no independent album record to rename atomically.
- Two visually similar Unicode titles may remain separate if their normalized comparison keys differ.
- Workspace-index loss discards album membership because portable track content intentionally excludes this organizational metadata.
- Version 0.1 does not provide manual track order within an album.

## Related documents

- [Getting started](../usr/getting-started.md)
- [Product architecture](product-architecture.md)
- [Persistence and recovery](persistence.md)
- [Track documentation model](track-documentation-model.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-14 | Defined the album and single library hierarchy, invariants, persistence boundary, and acceptance requirements. | Product team |
