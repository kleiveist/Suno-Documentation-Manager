<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Local persistence and recovery

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-14 |
| Audience | Product developers and recovery reviewers |
| Related ATP | [ATP-0011: Local persistence and recovery](../atp/active/ATP-0011-local-persistence-and-recovery.md); [ATP-0014: Track library organization](../atp/active/ATP-0014-track-library-organization.md) |

## Purpose

This document defines which local data store is authoritative, how SQLite and portable files cooperate, and how the application recovers after index loss or discovers a historical track folder.

## Scope

### Included

- workspace management storage;
- the product-specific local SQLite boundary;
- track-folder authority and relative-path rules;
- schema migration, transactional update, atomic file write, backup, and recovery responsibilities; and
- privacy, corruption, and partial-failure behavior.

### Excluded

- cloud synchronization, remote databases, remote backup, or multi-device conflict resolution;
- PostgreSQL, SQLAlchemy, and Alembic;
- storing large evidence blobs inside SQLite;
- automatic deletion of user evidence; and
- a universal persistence abstraction for unrelated products.

## Source-of-truth model

The product uses bounded authority rather than treating every copy as equally authoritative.

| Data set | Source of truth | Rebuild or snapshot rule |
| --- | --- | --- |
| Workspace profile and current defaults | `.suno-doc/workspace.sqlite` | A finalized track keeps its own snapshot and is not changed by later defaults |
| Track index, mutable workflow state, evidence metadata including provenance and derivation, and UI work state | `.suno-doc/workspace.sqlite` | Rebuilt as far as possible by scanning portable track files; unknown facts remain unknown |
| Album or single library placement | `.suno-doc/workspace.sqlite` track JSON | Missing or unrecoverable placement defaults to `single`; the portable track folder does not encode album membership |
| Imported evidence and release assets | Track folder | Database references store role and root-relative path; file contents are not database blobs |
| Generated documentation | Track folder | Regenerated only through explicit managed-document rules |
| Final hashes, manifest, certificate, and archived revisions | Track folder | Treated as the authoritative finalized snapshot |
| Registered reusable subscription evidence | `.suno-doc/global-evidence/` plus SQLite materialized coverage metadata | Selected evidence and its exact coverage interval are copied into a track before finalization so the track remains self-contained |

Deleting the database can lose unfinished form state or reusable global defaults. It must not make a complete finalized track folder impossible to inspect and verify.

## Workspace management area

After the user creates or opens a workspace, native code may create this reserved structure:

```text
<workspace>/.suno-doc/
├── workspace.sqlite
├── config/
└── global-evidence/
```

`.suno-doc/` is internal application state, not track evidence. It is excluded from track integrity lists and certificates. The application does not hardcode a machine-specific absolute workspace path in source or documentation. It stores root-relative paths where a durable path is necessary.

## SQLite ownership

`PersistenceService` owns connection creation, transaction boundaries, schema version checks, migrations, and controlled recovery errors. Rust services use typed repository operations. TypeScript never submits SQL and no command exposes `execute_sql` or an equivalent interface.

The database indexes at least these logical data sets:

- workspace metadata and global profile values;
- global-evidence records with their materialized per-invoice coverage dates;
- known tracks and their lifecycle status;
- each track's `single` or named-album library placement;
- workflow ID, workflow version, step states, applicability, and N/A reasons;
- evidence roles, relative paths, media types, sizes, hashes, import metadata, provenance, and local derivation fields;
- generated-document template versions and freshness markers; and
- UI working state that is not part of a finalized certificate.

Large media, PDFs, archives, and generated portable artifacts remain files. SQLite stores metadata and references, not duplicate binary evidence payloads.

## Schema and migrations

The current SQLite schema version is `2`, stored in `PRAGMA user_version`. Schema version 2 adds these columns to each track-evidence record:

| Column | Purpose |
| --- | --- |
| `provenance` | One of `managed_copy`, `global_copy`, `generated_disclosure`, or `indexed_legacy` |
| `derived_from_evidence_id` | Source evidence ID for a locally derived artifact |
| `generator_version` | Versioned native generator identity, currently `local-disclosure-v1` for visible artwork disclosure |
| `generated_disclosure_text` | Exact normalized text rendered into a generated disclosure artifact |

The version 1 to version 2 migration backfills evidence belonging to a track already marked `legacy` as `indexed_legacy`; all other old rows are conservatively `managed_copy`. It leaves derivation, generator, and generated-text fields empty because version 1 did not retain enough data to prove them. In particular, migration never upgrades an old row to `generated_disclosure` merely from its role, filename, or bytes.

Track records are serialized in the existing `tracks.data_json` column. The album-or-single placement is an additive typed JSON field with a Serde default of `single`. It adds no table, column, index, or relation, so introducing it does not advance `PRAGMA user_version` beyond `2`. Reading older JSON without the field is backward-compatible; a later record save materializes the default. This rule applies only to compatible additive fields with an explicit safe default. Relational layout changes still require an ordered schema migration.

Opening a supported older database performs ordered native migrations inside a transaction. A migration must satisfy these rules:

1. Verify the current version before changing data.
2. Back up or otherwise preserve recoverability before an irreversible schema change.
3. Apply one ordered migration at a time inside a transaction.
4. Update the recorded schema version only after the migration succeeds.
5. Roll back the transaction and return a controlled error when a step fails.
6. Refuse a database created by an unsupported newer application instead of guessing its meaning.

Migrations never rewrite track evidence as an incidental database-open side effect.

## Evidence provenance and portable lineage

Track evidence uses four ownership and origin categories:

| Provenance | Creation path | Portable meaning |
| --- | --- | --- |
| `managed_copy` | A user selects a source through native track import | The application copied and verified the file into a managed track destination |
| `global_copy` | A selected reusable workspace record is copied into the track | `sourceGlobalEvidenceId` and applicable coverage dates identify the workspace source record |
| `generated_disclosure` | The native artwork service renders a disclosure from verified AI-original evidence | `derivedFromEvidenceId`, `generatorVersion`, and `generatedDisclosureText` establish the local process lineage |
| `indexed_legacy` | A workspace scan discovers a pre-existing file | The application indexed the observed historical file but does not claim that it originally imported or generated it |

SQLite keeps these fields while a track is being managed. At finalization, every verified evidence entry in `06_CERTIFICATE/EVIDENCE_MANIFEST.json` includes its provenance and any applicable lineage fields. Manifest paths remain track-root-relative. This makes the evidence origin reviewable without the workspace database.

The AI disclosure gate requires more than an `ai_artwork_edited` role. The artifact must have `generated_disclosure` provenance, generator version `local-disclosure-v1`, the exact current disclosure text, and a `derivedFromEvidenceId` that identifies present verified AI-original evidence. The final artwork must also have the same SHA-256 digest as that locally generated artifact.

## Coordinating database and files

SQLite transactions and filesystem writes do not share one atomic transaction. Each use case therefore uses an order that leaves recoverable evidence:

### Evidence import

1. Validate the selected source and evidence role.
2. Resolve and validate the contained destination.
3. Detect a collision before writing.
4. Copy to a temporary destination without removing the source.
5. Calculate and verify the copied file hash and size.
6. Rename into place.
7. Commit the evidence metadata in SQLite.
8. Reevaluate workflow state.

If metadata commit fails after the file is placed, a later scan can discover the unindexed file. The product reports the recoverable mismatch rather than deleting user evidence automatically.

### Indexed legacy evidence removal

Removing `indexed_legacy` evidence is an explicit recoverable operation rather than ordinary deletion. If the indexed file is present, the application moves it to a unique `.archive/removals/<removal-id>/` directory, writes `removal.json` with the original relative path and evidence metadata, and only then removes the SQLite evidence row. A missing historical file still receives the removal record before de-indexing.

If metadata or track-state persistence fails, the operation attempts to move the file and evidence row back. After success the original path is absent, `.archive/` is outside later legacy discovery, and rescanning cannot silently re-add the removed evidence. Version 0.1 exposes the archive for manual recovery and audit; it does not claim an automatic restore command.

### Generated document

1. Normalize and validate the input snapshot.
2. Render a versioned template in memory.
3. If an unmanaged destination exists, require explicit adoption and create a backup.
4. Write a temporary sibling file.
5. Flush and rename atomically.
6. Record the template version and freshness metadata.

A crash leaves either the previous managed document or the complete new document. Recovery reports and safely removes or quarantines an identifiable stale temporary file only through a documented native operation.

### Finalization

Finalization validates all current portable inputs, renders the three certificate artifacts into a contained staging directory, verifies them, and publishes the complete directory before committing the `FINALIZED` index state. It then verifies the protected track set again so an external edit during publication cannot produce a valid status. If the database commit fails, the native service rolls the published set back. An incomplete certificate set must not be presented as finalized.

Before publishing a certificate set, finalization writes `.archive/finalization-in-progress.json` with the track, certificate, transaction, and start identifiers. Workspace open uses this marker to distinguish an interrupted application transaction from historical certificate content. A finalized database record with a stale marker removes the marker; a non-finalized matching record with published certificate files moves only that interrupted set below `.archive/recovery/<transaction-id>/certificate/` and writes recovery metadata. Finalization removes the marker after the SQLite `FINALIZED` commit on a best-effort basis.

A non-finalized or imported legacy track whose `06_CERTIFICATE/` contains historical files but has no application-created transaction marker remains untouched. The recovery path does not quarantine a certificate directory merely because the SQLite status is non-finalized. Separately, a finalized database record with a temporarily moved certificate directory can be restored from revision staging or its matching revision archive.

## Relative paths and containment

The workspace record can remember the user-selected root for the local application instance, but persisted track and manifest references use normalized paths relative to their owning root. Inputs containing absolute paths, `..`, invalid components, or symbolic-link components observed during validation are rejected before a write.

These checks are path-based. They do not make the full check-and-use sequence race-free against another process running as the same operating-system user and able to modify the workspace. Such shared writable workspaces are outside the version 0.1 threat model; descriptor-relative hardening remains open in [ATP-0012](../atp/active/ATP-0012-filesystem-containment.md).

Moving a complete track folder inside a workspace does not require its internal manifest paths to change. Managed tracks store their stable ID in `.summary/track.json`; this excluded marker lets reopen or scan reconcile a renamed track or album path with SQLite. Moving an entire workspace can be recovered by opening the new root and scanning it.

## Recovery and reindexing

`scan_workspace` treats the filesystem as untrusted input. The scan never changes candidate track files, but it does add or reconcile local SQLite index records so discovered tracks become visible immediately. It:

1. ignores `.suno-doc/` as a track;
2. recognizes historical direct-child tracks, `Singles/<track>/`, and `<album>/<track>/`;
3. records known managed-document paths, a historical hash-list presence flag, and contained evidence candidates;
4. adds evidence candidates with inferred roles but preserves their historical provenance as `NOT VERIFIED`;
5. reconciles newly discovered evidence on a later scan without duplicating existing index entries; and
6. derives `single` from the reserved `Singles/` parent and an album placement from a named album parent while defaulting historical direct-child tracks to `single`; and
7. leaves profile and track facts unknown until the user supplies them or explicitly confirms adoption of the current workspace profile as the track snapshot.

Scanning and reindexing never overwrite an existing track file. Explicit evidence verification confirms present bytes, but it does not manufacture historical provenance. Adopting an existing document requires a preview, explicit user confirmation, a backup below `.archive/`, and only then a managed write. See [Legacy track import](../dev/legacy-track-import.md).

Creating, reclassifying, or renaming managed library folders is a separate explicit mutation path. It collision-checks every destination, moves the complete directory, persists all affected relative paths transactionally, and performs a compensating move if persistence fails. These operations do not rewrite anything below the moved track roots.

Files already moved into `.archive/removals/` by an explicit legacy-evidence removal are excluded from discovery. Their `removal.json` records retain the original relative path and evidence metadata so they remain recoverable without making the next scan undo the user's choice.

## Global evidence

Reusable Suno subscription or payment evidence can be registered under `.suno-doc/global-evidence/`. Each registration represents exactly one selected evidence file (PDF, PNG/JPEG, TXT, or Markdown) and one evidenced billing period; a folder or multi-file selection does not become one combined record. The source is copied without being moved or deleted.

The user selects the cadence shown by that invoice (`monthly` or `annual`) and enters its factual coverage start date. The application derives one concrete inclusive coverage end for that record: the day before the next monthly payment date for `monthly`, or the day before the payment date twelve calendar months later for `annual`. It persists the resulting start and end dates rather than leaving the end as a calculation that may change later. The durable evidence fact is this materialized interval, not an instruction to repeat the cadence. The account-level subscription start date is a separate profile fact and is not substituted for the invoice coverage start.

Cadence never creates recurring evidence or open-ended coverage. One monthly invoice evidences only its one materialized month, and one annual invoice evidences only its one materialized year. Additional periods require separately registered source invoices. A record must not be treated as covering cancellation, refund, partial-period, or later-renewal dates that the selected document does not actually support.

A track selects a record only when its concrete interval covers the applicable production period. Before finalization the selected file is copied into the track evidence structure, collision-checked, hashed, marked `global_copy`, and linked to its workspace source record. Its exact `coverageStart` and `coverageEnd` values are included in the portable manifest with the track-relative file path. Recovery and review therefore use the materialized dates and do not extrapolate from cadence or require the workspace database to recalculate coverage.

Private email addresses, telephone numbers, birthdays, account credentials, and unrelated account details are neither required global fields nor copied into generated license documentation.

## Backup and revision behavior

The application does not silently mutate a finalized snapshot. When an included file changes, integrity verification marks the certificate invalid. `create_revision` archives the prior main hash list, certificate directory, and revision metadata as follows, even when the certificate set is already damaged or incomplete:

```text
.archive/revisions/<revision-id>/
├── revision.json
├── 03_DOCUMENTATION/
│   └── SHA256SUMS.txt
└── certificate/
    ├── DOCUMENTATION_CERTIFICATE.md
    ├── EVIDENCE_MANIFEST.json
    └── CERTIFICATE_SHA256.txt
```

The archived revision remains outside normal managed writes. Revision creation moves the live certificate directory as one directory and rolls it back if the SQLite status update fails. A new working revision can then update facts, evidence, documents, hashes, and certificate artifacts through the ordinary gate.

User-managed workspace backup is a copy of the entire workspace, including `.suno-doc/`. A track-only backup remains portable but may omit unfinished UI state and reusable global defaults. The application does not provide remote backup in version 0.1.

Album membership is another workspace-index-only value. A track-only copy retains every portable evidence and certificate artifact but not its library placement. Reopening a complete workspace preserves the placement; scanning a track after index loss conservatively places it under `Singles`.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-PER-001` | Workspace creation initializes a usable local management area and SQLite schema. | [ATP-0001](../atp/active/ATP-0001-workspace-creation-and-loading.md) |
| `REQ-PER-002` | Rust owns typed SQLite access; no frontend raw-SQL capability exists. | [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) |
| `REQ-PER-003` | Ordered migrations are transactional and a failed migration does not advance the schema version. | [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) |
| `REQ-PER-004` | Deleting an index allows a track scan to recover only evidenced facts and preserve unknowns. | [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) |
| `REQ-PER-005` | Global evidence selected for finalization is copied into and hashed with the portable track, and its exact materialized coverage dates are retained in the portable manifest. | [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) |
| `REQ-PER-006` | All durable portable references are track-root-relative and contain no local absolute path. | [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) |
| `REQ-PER-007` | Evidence provenance and local disclosure lineage survive SQLite persistence and portable-manifest generation without being inferred from a role alone. | [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) |
| `REQ-PER-008` | Each global subscription registration selects exactly one source file and materializes one inclusive coverage interval from its factual start and monthly/annual cadence; cadence is not recurring evidence. | [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) |

## Verification

The acceptance owner uses temporary workspaces and identified builds. Verification commands, run from the repository root, are:

```sh
python tools/control.py test --suite tauri
python tools/control.py test --suite all --report
rg -n "execute_sql|DATABASE_URL|postgres" frontend src-tauri
```

Executed and outstanding recovery, migration-failure, and index-loss results are recorded in [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) and the [acceptance report](../dev/acceptance-report.md).

## Risks and limitations

- Index recovery cannot reconstruct facts that were never exported to portable files; those facts remain `NOT VERIFIED`.
- Filesystem and SQLite operations require explicit compensation and scan logic because they are not one atomic transaction.
- A track-only backup is not a complete backup of unfinished workspace settings.
- A track-only backup or index-loss scan cannot recover album membership and defaults the recovered track to `single`.
- Copying global evidence into multiple tracks deliberately duplicates files to preserve track portability.
- The version 1 migration cannot reconstruct derivation that was not stored; non-legacy version 1 evidence is therefore backfilled conservatively as `managed_copy`.
- Path containment is not descriptor-relative across the complete operation and does not protect a workspace from a hostile same-user concurrent writer.

## Related documents

- [Product architecture](product-architecture.md)
- [Track documentation model](track-documentation-model.md)
- [Track library organization model](track-library-model.md)
- [Workflow model](workflow-model.md)
- [Legacy track import](../dev/legacy-track-import.md)
- [Provider-neutral template persistence guidance](persistence-architecture.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-14 | Defined workspace-index ownership, additive JSON compatibility, and index-loss behavior for track library placement. | Project team |
| 2026-08-14 | Defined per-invoice cadence, single-file registration, materialized coverage dates, portability, and the no-extrapolation boundary. | Project team |
| 2026-08-13 | Documented schema version 2, evidence provenance and disclosure lineage, recoverable legacy removal, and marker-based finalization recovery. | Project team |
| 2026-08-13 | Aligned scan, revision archive, and interrupted-operation recovery behavior with version 0.1. | Project team |
| 2026-08-13 | Defined SQLite ownership, portable-file authority, and recovery behavior. | Project team |
