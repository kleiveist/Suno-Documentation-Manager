<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Local persistence and recovery

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-20 |
| Audience | Product developers and recovery reviewers |
| Related ATP | [ATP-0011: Local persistence and recovery](../atp/active/ATP-0011-local-persistence-and-recovery.md); [ATP-0014: Track library organization](../atp/active/ATP-0014-track-library-organization.md); [ATP-0017: Pre-release audio screening](../atp/active/ATP-0017-pre-release-audio-screening.md) |

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
| Pre-release local screening records and optional provider-response archive | Track folder plus mutable track JSON state | The portable artifacts are phase-one SHA-256 inputs; raw fingerprints are not copied into certificates/manifests |
| Non-secret optional ACRCloud settings | `.suno-doc/workspace.sqlite` | Global workspace operational state, never copied into profile or track snapshots |
| ACRCloud credentials | `.suno-doc/config/audio-screening-secrets.json` | Private write-only configuration, excluded from exports and SQLite |
| Final hashes, manifest, certificate, timestamp addenda, and archived revisions | Track folder | Treated as the authoritative finalized snapshot and revision-bound post-finalization records |
| Registered reusable subscription and Suno terms evidence | `.suno-doc/global-evidence/` plus SQLite coverage and factual descriptive metadata | Evidence is copied into each applicable track before finalization so the track remains self-contained |

Deleting the database can lose unfinished form state or reusable global defaults. It must not make a complete finalized track folder impossible to inspect and verify.

Saving the workspace profile and the affected non-finalized track records is one SQLite transaction. Each open track receives the changed profile snapshot, generated-document freshness and integrity state are reset, and the workflow is reevaluated. Workspace opening also reconciles older non-finalized records against the already stored profile. Finalized and superseded track snapshots are excluded from this synchronization.

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
- global-evidence records with materialized per-invoice coverage dates or Suno Terms title/provider/retrieval and optional contextual metadata;
- known tracks and their lifecycle status;
- each track's `single` or named-album library placement;
- workflow ID, workflow version, step states, applicability, and N/A reasons;
- evidence roles, relative paths, media types, sizes, hashes, import metadata, provenance, and local derivation fields;
- generated-document template versions and freshness markers;
- external-timestamp records keyed to their track and finalized certificate ID; and
- non-secret optional ACRCloud screening settings, while its credential pair remains in a restricted private configuration file; and
- UI working state that is not part of a finalized certificate.

Large media, PDFs, archives, and generated portable artifacts remain files. SQLite stores metadata and references, not duplicate binary evidence payloads.

## Schema and migrations

The current SQLite schema version is `7`, stored in `PRAGMA user_version`. Schema version 2 added evidence provenance and disclosure-lineage columns. Schema version 3 added `metadata_json` to each track-evidence record for original import filename and role-specific metadata. Schema version 4 added the same conservative field to workspace-global evidence. Existing rows received an empty object; no historical title, provider, date, or source was invented. Schema version 5 adds `external_timestamp_records`, keyed by timestamp-record ID and indexed by track ID, certificate ID, import time, and record ID. Schema version 6 adds singleton timestamp settings and current attachment presentation state. Schema version 7 adds singleton non-secret `audio_screening_settings`. The complete typed timestamp record is retained as JSON while the identifying columns support deterministic revision-bound lookup.

| Column | Purpose |
| --- | --- |
| `provenance` | One of `managed_copy`, `global_copy`, `generated_disclosure`, or `indexed_legacy` |
| `derived_from_evidence_id` | Source evidence ID for a locally derived artifact |
| `generator_version` | Versioned native generator identity, currently `local-disclosure-v1` for visible artwork disclosure |
| `generated_disclosure_text` | Exact normalized text rendered into a generated disclosure artifact |

The version 1 to version 2 migration backfills evidence belonging to a track already marked `legacy` as `indexed_legacy`; all other old rows are conservatively `managed_copy`. It leaves derivation, generator, and generated-text fields empty because version 1 did not retain enough data to prove them. In particular, migration never upgrades an old row to `generated_disclosure` merely from its role, filename, or bytes.

Track records are serialized in the existing `tracks.data_json` column. The current Suno semantics are nullable scalar `sunoContentClassification` and `vocalIntent` values serialized as canonical SCREAMING_SNAKE_CASE tokens. The former content Boolean and multi-value array are read-only compatibility data and are omitted after a canonical choice or successful explicit upgrade/revision migration. Vocal Intent has no inferred or migrated default.

The former `lyricsSource` and `lyricsText` properties are read through legacy aliases and emitted under explicit legacy names. Their presence does not populate `vocalLyricsPresent`, `vocalIntent`, `sunoContentClassification`, or content source. Missing new answers deserialize as unknown/`NOT DOCUMENTED`, and no migration scans free text or bracketed instructions to infer meaning.

The former `sunoPlanAtCreation` property is read only into the separate `legacySunoPlanAtCreation` compatibility value. It is not an alias for, copied into, or rendered as the new `sunoPlanAtGeneration` fact. For an older record without an explicitly confirmed `sunoPlanAtGeneration`, the plan at generation remains empty/`NOT DOCUMENTED` and continues to block any requirement that needs it until the user confirms the value in mutable state. The legacy value remains visible as historical user data without asserting that it was the plan used for the final generation. Reading an older record is therefore non-destructive; a later mutable save can materialize only answers actually supplied by the user. Finalized and superseded records are never rewritten merely to adopt the new names.

Most of these JSON additions require no relational migration. Schema 5 advances `PRAGMA user_version` because post-finalization timestamp records need their own certificate-bound table rather than entering the immutable pre-finalization track snapshot. Schema 6 adds non-secret global timestamp settings plus per-track/Certificate-ID attachment-status presentation state; its password/token remains outside SQLite in restricted `.suno-doc/config/timestamp-secrets.json`. Schema 7 adds non-secret ACRCloud settings; its access key/secret pair likewise remains in `.suno-doc/config/audio-screening-secrets.json`. Both private files are covered by the workspace credential `.gitignore` rule. Local screening state is an additive defaulted field in mutable track JSON, and older finalized snapshots are not rewritten or backfilled. Existing ordinary evidence rows that used the historical external-timestamp role are not silently promoted to the certificate-bound table or assigned to a certificate.

An active managed final-audio record at the exact historical `01_RELEASE/suno_final_export.<supported extension>` path can be migrated lazily to the safe title-based filename when the target is unused. The operation updates the file and evidence path together and marks generated documents stale. It never applies to finalized or superseded snapshots, indexed legacy provenance, ambiguous paths, or collisions.

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
4. Stream once from the source into a temporary destination while calculating SHA-256 and size, without removing the source.
5. Flush the complete copied file.
6. Rename into place.
7. Inspect technical metadata from that managed copy, so its WAV facts and the stored hash describe the same bytes.
8. Reevaluate the derived track state and commit the evidence row plus track record in one SQLite transaction.

If validation or the combined database commit fails after the file is placed, the managed copy is removed and neither the evidence row nor its derived track facts are committed. The selected source remains untouched.

Routine list and load operations bound their work for evidence larger than 64 MiB: they check containment, file kind, existence, and stored size but do not recalculate the full digest. Explicit evidence verification, integrity generation/verification, and finalization still read and cryptographically verify the full file set.

An explicit replacement resolves one existing evidence ID. The new source is streamed and hashed into a staged managed file, technical metadata is inspected from that copy, and the previous bytes move to `.archive/evidence-replacements/<transaction-id>/`. SQLite updates the existing evidence row and its derived track state in one transaction. Persistence failure removes the replacement and restores the archived file while the old database state remains intact. A normal import that resolves to an already indexed `(track_id, relative_path)` is rejected before copying with a controlled instruction to use replacement, preventing the database uniqueness failure from leaking into the UI.

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

### Pre-release audio screening

The authoritative release evidence is the sole source for local screening. On import or explicit replacement of that editable `release_wav` evidence, the application clears or marks prior screening state stale, runs the packaged verified Chromaprint engine against the contained managed file, writes the new local JSON, detached local-record SHA-256, and Markdown summary atomically below `03_DOCUMENTATION/AUDIO_SCREENING/`, and persists the matching state with source Evidence ID/path/SHA-256 in the same tracked update path. A persistence failure must not leave a state that claims a new source while only the old local artifact is live; recovery keeps the prior bytes or reports the incomplete result as non-positive.

The optional ACRCloud operation is a separate explicit mutable-track action. It first reads the write-only credential pair from the private workspace configuration, builds a bounded sample from the managed release bytes, performs one bounded HTTPS request, and atomically writes the structured external record plus only a safe raw provider response under `03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_SCREENING.json` and `ACRCLOUD_RESPONSE.json`. It persists the provider status, source binding, sample timings, response artifact path/hash, and concise provider match facts, but never a credential or request signature. A disabled setting, incomplete credentials, network failure, unsupported input, or provider error is a stored non-positive technical result. This operation is not retried at startup and is never called by finalization.

`03_DOCUMENTATION/AUDIO_SCREENING/` is ordinary current documentation rather than `EvidenceRole` data. Its live files are included in `SHA256SUMS.txt`; archive copies are excluded with the rest of `.archive/`. A new revision first retains the previous screening directory with its prior phase-one snapshot and begins a fresh mutable screening state for its own release source.

### Finalization

Finalization validates all current portable inputs, renders the three certificate artifacts into a contained staging directory, verifies them, and publishes the complete directory before committing the `FINALIZED` index state. It then verifies the protected track set again so an external edit during publication cannot produce a valid status. If the database commit fails, the native service rolls the published set back. An incomplete certificate set must not be presented as finalized.

Before publishing a certificate set, finalization writes `.archive/finalization-in-progress.json` with the track, certificate, transaction, and start identifiers. Workspace open uses this marker to distinguish an interrupted application transaction from historical certificate content. A finalized database record with a stale marker removes the marker; a non-finalized matching record with published certificate files moves only that interrupted set below `.archive/recovery/<transaction-id>/certificate/` and writes recovery metadata. Finalization removes the marker after the SQLite `FINALIZED` commit on a best-effort basis.

A non-finalized or imported legacy track whose `06_CERTIFICATE/` contains historical files but has no application-created transaction marker remains untouched. The recovery path does not quarantine a certificate directory merely because the SQLite status is non-finalized. Separately, a finalized database record with a temporarily moved certificate directory can be restored from revision staging or its matching revision archive.

### External timestamp attachment

External timestamp attachment is a second, post-commit phase. It is neither an ordinary track-evidence import nor part of the base certificate transaction. Phase one first publishes and commits the complete `FINALIZED` snapshot. Only then may a configured automatic action or a later explicit action contact a provider. Any phase-two provider, verification, staging, or publication failure is stored as timestamp status where possible and never rolls back `DOCUMENTATION COMPLETE` or changes a phase-one byte.

The automatic RFC-3161 path has one non-user-selectable anchor and verification contract:

1. Require a valid `FINALIZED` track, verify its certificate and main integrity set, and bind the attempt to both its Certificate ID and immutable finalization-snapshot ID.
2. Resolve `06_CERTIFICATE/EVIDENCE_MANIFEST.json` from `CERTIFICATE_SHA256.txt`, reread its exact bytes, and require their SHA-256 to equal that phase-one entry before the request. The same path and digest are checked again immediately after the network response and after sidecar publication.
3. Build an RFC-3161 SHA-256 request containing a fresh random nonce and, for a custom TSA, the configured optional policy OID. The UI cannot substitute another artifact or digest on this path.
4. Accept a positive response only when TimeStampResp status is `granted` or `grantedWithMods`, TSTInfo is version 1, the SHA-256 message imprint equals the exact manifest digest, the response nonce exactly equals the request nonce, a policy OID is returned, and that OID equals the requested policy when one was configured.
5. Cryptographically verify the CMS signed attributes and signature using the declared supported algorithm; require the TSA signer certificate's Extended Key Usage extension to be critical and contain only `id-kp-timeStamping`; require the signing chain to be valid at the response `genTime`; and build that chain to one or more certificates from the explicitly configured TSA CA trust-anchor file. The immutable record retains the verifier identifier and SHA-256 fingerprints of the configured roots. No operating-system or downloaded implicit trust store is substituted.
6. Archive the untouched `.tsr` response, provider-derived metadata, exact request/response binding results, Markdown addendum, PDF/A-2b addendum, and their hashes in the separate sidecar. `VERIFIED` is available only when every structural, imprint, nonce, policy, signature, EKU/time, chain, anchor, and sidecar-integrity predicate is positive.

The RFC-3161 CMS verifier is the locally vendored `sigstore-tsa` 0.10.0 base from `prefix-dev/sigstore-rust` commit `2501a347c5c858bb91feb96f40f8eb67f06d6418`, with the application verifier label identifying the local RSA/algorithm-dispatch and strict-EKU patch. Its Apache-2.0 provenance and patch scope are recorded in [`README-SUNODM.md`](../../src-tauri/vendor/sigstore-tsa/README-SUNODM.md).

The manual/legacy attachment path remains deliberately different. The user selects a stable artifact, provides an external evidence file and descriptive provider/type/time/reference values, and SunoDM recalculates the local artifact SHA-256. A custom `Other` artifact is eligible only when its exact relative path and current digest are already an unchanged entry in the verified phase-one `03_DOCUMENTATION/SHA256SUMS.txt`. A claimed mismatch remains visible as `Referenced hash match: NO`; this path does not automatically verify provider identity, CMS signature, TSA EKU, or a certificate chain and is never promoted to RFC-3161 `VERIFIED`. An initial OpenTimestamps calendar result is wrapped as a detached `.ots` proof bound locally to the requested SHA-256 and remains `ATTACHED` until a separate OTS verification or upgrade exists; it is not represented as an RFC-3161 result.

Both paths use sidecar format v1 and the same durable stage → database registration → live publication transaction. The operation writes immutable `TIMESTAMP_RECORD.json`, the managed timestamp evidence, `EXTERNAL_TIMESTAMP_ADDENDUM.md`, `EXTERNAL_TIMESTAMP_ADDENDUM.pdf`, and `TIMESTAMP_RECORD_SHA256.txt`; a provider adapter may additionally retain one exact raw `PROVIDER_RESPONSE.<ext>` file when the usable evidence is a derived wrapper. The record pins `markdownSha256`, `pdfSha256`, provider-response filename/hash when present, and `integrityVerifiedAtPublication`, while current `integrityVerified` and `integrityIssues` remain presentation state. Every regular file and directory boundary is synchronized before registration/publication where the platform supports it. A compensating removal synchronizes the live parent before deleting the SQLite row; otherwise the registration remains available for recovery.

The base `EVIDENCE_MANIFEST.json`, `SHA256SUMS.txt`, Markdown certificate, both PDF certificates, and `CERTIFICATE_SHA256.txt` remain byte-identical. This ordering avoids a cycle in which adding timestamp evidence would change the artifact being timestamped. The timestamp record's own integrity list protects the addendum instead.

Workspace startup reconciles timestamp publication by database identity. A valid stage with a matching registered row is verified and published; an unregistered abandoned stage is removed. An unexpected live sidecar without a registered row is rejected and never auto-adopted. A registered sidecar already located in exactly one managed revision archive is treated as durably published there and is never restored into the current revision.

Whenever a timestamped track is loaded, every registered current or archived record is independently resolved and sidecar-reverified. Reverification requires the exact managed regular-file set described by the record; byte-for-byte equality with the canonical immutable v1 JSON registered by SQLite; SHA-256 verification of the exact published record, evidence, optional raw provider response, Markdown, and PDF bytes; equality with pinned provider-response/Markdown/PDF hashes; an unchanged referenced phase-one or archived artifact with the correct stored match result; and an exact versioned `TIMESTAMP_RECORD_SHA256.txt`. It deliberately hashes published bytes instead of re-rendering historical addenda. An archived location additionally requires `revision.json.previous_certificate.certificateId` to equal the sidecar Certificate ID.

Load-time sidecar verification does not silently rerun a network request or reinterpret legacy evidence. For an automatic RFC-3161 record, the positive summary is reconstructed only when the exact immutable verified provider metadata is still covered by the intact sidecar, the referenced artifact is specifically the finalized Evidence Manifest with equal claimed/actual SHA-256, nonce and policy predicates remain complete, signature and pinned-chain results are positive, the strict versioned verifier identifier matches, and all retained trust-anchor fingerprints are well formed. Missing, stale, tampered, or merely summary-only `VERIFIED` state is downgraded to a visible failure. A damaged sidecar remains associated with its record but does not rewrite or invalidate the independent base certificate. Legacy format-v0 sidecars remain self-consistency verifiable without renderer equality and are not silently rewritten as v1.

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

Reusable Suno subscription/payment evidence and archived Suno terms/rights evidence can be registered under `.suno-doc/global-evidence/`. Each registration represents exactly one selected file; a folder or multi-file selection does not become one combined record. Subscription records retain one evidenced billing period. Terms registration accepts only a PDF whose bytes carry the PDF signature and retains its user-confirmed document title, provider/source, and retrieval date. Source URL, effective date, applicable production period, and a factual note are optional. Original filename, managed path, SHA-256, import time, and provenance remain system/evidence facts. The source is copied without being moved or deleted, and no value is fetched from a network.

Global terms are automatically copied into every newly created or currently editable track with `global_copy` provenance and `sourceGlobalEvidenceId`. Updating descriptive metadata on the global record propagates the exact metadata to matching portable copies only while their tracks remain editable and marks affected documents/integrity stale. Existing finalized and superseded snapshots are not mutated; a subsequent revision can receive newer or corrected terms. A manual per-track attach remains available as an idempotent recovery path.

For a commercially intended track, file presence alone is insufficient: document title, provider/source, and retrieval date must be non-empty. An older record with empty migrated metadata stays readable and blocks commercial completion until the user supplies those facts. Source URL and effective date may honestly remain `NOT DOCUMENTED`. The application records applicable periods and notes without deciding that Terms are valid, enforceable, or sufficient for commercial rights.

Terms availability is an invariant, not a display preference. If a verified local `suno_terms_rights` record with a SHA-256 is attached, the native track-update API rejects a request to set `Terms evidence not available` to `YES` and preserves the prior values. If imported legacy state already contains that contradiction, workflow consistency reports a blocking `terms_evidence_availability_conflict`; Markdown and PDF certificate generation also reject the contradictory snapshot instead of publishing both statements.

The user selects the cadence shown by that invoice (`monthly` or `annual`) and enters its factual coverage start date. The application derives one concrete inclusive coverage end for that record: the day before the next monthly payment date for `monthly`, or the day before the payment date twelve calendar months later for `annual`. It persists the resulting start and end dates rather than leaving the end as a calculation that may change later. The durable evidence fact is this materialized interval, not an instruction to repeat the cadence. The account-level subscription start date is a separate profile fact and is not substituted for the invoice coverage start.

Cadence never creates recurring evidence or open-ended coverage. One monthly invoice evidences only its one materialized month, and one annual invoice evidences only its one materialized year. Additional periods require separately registered source invoices. A record must not be treated as covering cancellation, refund, partial-period, or later-renewal dates that the selected document does not actually support.

A track can attach an individually relevant record when its concrete interval overlaps production or contains final generation. Before finalization the selected file is copied into the track evidence structure, collision-checked, hashed, marked `global_copy`, and linked to its workspace source record. Its exact `coverageStart` and `coverageEnd` values are included in the portable manifest with the track-relative file path. The native gate then joins adjacent selected intervals and requires gap-free coverage of the production period plus at least one interval containing final generation. Recovery and review therefore use the materialized dates and do not extrapolate from cadence or require the workspace database to recalculate coverage. This is a date comparison only, not a rights or license-validity conclusion.

Private email addresses, telephone numbers, birthdays, account credentials, and unrelated account details are neither required global fields nor copied into generated license documentation.

## Backup and revision behavior

The application does not silently mutate a finalized snapshot. When an included file changes, integrity verification marks the certificate invalid. `create_revision` archives the prior main hash list, certificate directory, and revision metadata as follows, even when the certificate set is already damaged or incomplete:

```text
.archive/revisions/<revision-id>/
├── revision.json
├── SunoDM_DOCUMENTATION_CERTIFICATE.pdf
├── SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf
├── 03_DOCUMENTATION/
│   └── SHA256SUMS.txt
└── certificate/
    ├── DOCUMENTATION_CERTIFICATE.md
    ├── EVIDENCE_MANIFEST.json
    ├── CERTIFICATE_SHA256.txt
    └── EXTERNAL_TIMESTAMPS/       # present only when attached to this revision
        └── <timestamp-record-id>/
```

The archived revision remains outside normal managed writes. Revision creation moves the live certificate directory, including all external-timestamp addenda, as one directory and rolls it back if the SQLite status update fails. Timestamp records stay associated with the archived certificate ID and are never reassigned to the new working revision. A new working revision can then update facts, evidence, documents, hashes, and certificate artifacts through the ordinary gate and may receive its own optional timestamp only after refinalization.

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
| `REQ-PER-009` | Global Suno Terms registration accepts one signature-checked PDF with title, provider/source, retrieval date, and optional factual context, and places the same hashed evidence/metadata into each new or editable project without mutating finalized snapshots. Verified local Terms evidence cannot be persisted or rendered together with an unavailable claim. | [ATP-0016](../atp/active/ATP-0016-evidence-certificate-workflow-5.md) |
| `REQ-PER-010` | Schema 5 persists post-finalization timestamp records separately by track and certificate ID; schema 6 adds non-secret provider settings and certificate-bound attachment-status presentation state while secrets stay outside SQLite. Sidecar v1 is synchronized before registration/publication and startup recovers only registered state. Automatic RFC-3161 is fixed to the finalized Evidence Manifest and can report `VERIFIED` only for a matching imprint, nonce, policy contract, CMS signature, strict TSA EKU/time validity, explicitly pinned chain, and intact current sidecar; manual/legacy/OTS evidence is not promoted. | [ATP-0016](../atp/active/ATP-0016-evidence-certificate-workflow-5.md) |
| `REQ-PER-011` | Legacy lyrics and plan-at-creation fields remain readable but never infer vocal/Suno-field semantics or satisfy the separate plan-at-generation fact; missing new semantics stay `NOT DOCUMENTED`. | [ATP-0016](../atp/active/ATP-0016-evidence-certificate-workflow-5.md) |
| `REQ-PER-012` | Schema 7 stores only non-secret optional ACRCloud settings; credentials remain private, screening artifacts are portable/hash-covered, and existing finalized snapshots are not backfilled. | [ATP-0017](../atp/active/ATP-0017-pre-release-audio-screening.md) |

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
- Timestamp sidecar directory-entry durability uses explicit parent-directory `fsync` on Unix. Non-Unix platforms retain atomic, synchronized file writes but directory synchronization is best effort.
- Timestamp-provider and ACRCloud credential protection relies on the local user account and restricted private config files; portable folders and SQLite intentionally cannot reconstruct credentials.

## Related documents

- [Product architecture](product-architecture.md)
- [Track documentation model](track-documentation-model.md)
- [Track library organization model](track-library-model.md)
- [Workflow model](workflow-model.md)
- [Pre-release audio screening](pre-release-audio-screening.md)
- [Legacy track import](../dev/legacy-track-import.md)
- [Provider-neutral template persistence guidance](persistence-architecture.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-20 | Defined the post-commit fixed-manifest RFC-3161 path, complete cryptographic verification predicate with explicit TSA trust anchors, manual/legacy/OTS separation, and exact sidecar-based reconstruction of current status. | Project team |
| 2026-08-18 | Raised SQLite schema to 7 for non-secret audio-screening settings and documented private credentials, portable screening artifacts, and revision isolation. | Project team |
| 2026-08-17 | Documented sidecar-v1 immutable byte pinning, stage-before-registration publication and startup reconciliation, current/archived byte-based reverification, and the native Terms availability invariant. | Project team |
| 2026-08-17 | Clarified that legacy plan-at-creation never populates plan at generation, restricted `Other` timestamp anchors to unchanged main-hash entries, and defined independent load-time sidecar integrity reporting. | Project team |
| 2026-08-17 | Raised SQLite schema to 5 for certificate-bound external-timestamp records and documented their staged sidecar transaction, stable anchors, mismatch persistence, revision isolation, enriched Terms metadata, and non-inferential lyrics migration. | Project team |
| 2026-08-16 | Documented additive workflow-field compatibility and the conservative managed release-name migration without a SQLite schema bump. | Project team |
| 2026-08-14 | Defined workspace-index ownership, additive JSON compatibility, and index-loss behavior for track library placement. | Project team |
| 2026-08-14 | Defined per-invoice cadence, single-file registration, materialized coverage dates, portability, and the no-extrapolation boundary. | Project team |
| 2026-08-13 | Documented schema version 2, evidence provenance and disclosure lineage, recoverable legacy removal, and marker-based finalization recovery. | Project team |
| 2026-08-16 | Documented schema version 3 and conservative empty metadata migration for terms/timestamp and original-filename facts. | Project team |
| 2026-08-16 | Raised the schema to version 4 for workspace-global Suno terms compatibility data and documented automatic portable project copies. | Project team |
| 2026-08-13 | Aligned scan, revision archive, and interrupted-operation recovery behavior with version 0.1. | Project team |
| 2026-08-13 | Defined SQLite ownership, portable-file authority, and recovery behavior. | Project team |
