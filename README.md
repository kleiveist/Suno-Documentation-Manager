# Suno Documentation Manager

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-20 |
| Audience | Users, contributors, and acceptance owners |
| Related ATP | [Product acceptance plans](docs/atp/active/active.md) |

## Purpose

Suno Documentation Manager is a local desktop application for creating a portable, verifiable documentation set for a music track. It guides the user through only the relevant questions, associates real evidence files, generates factual Markdown and text documents, verifies file integrity with SHA-256, and creates a Track Documentation Completion Certificate after every mandatory check passes.

The certificate confirms completion of the configured documentation and integrity workflow. It is not governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.

## Scope

### Included in version 0.1

- a German-language desktop interface built with Vite, TypeScript, and Tauri 2;
- local workspace selection and creation;
- a track library organized under permanent album and single sections;
- reusable global Suno and artwork settings with track-specific snapshots;
- a ten-step track documentation workflow;
- safe local evidence import without deleting or silently replacing source files;
- bounded extraction of structured Suno creation metadata and technical audio facts from imported WAV evidence;
- explicit evidence provenance for managed imports, global copies, locally generated disclosure outputs, and indexed legacy files;
- separate factual models for vocal lyrics, the Suno lyrics/structure field, audio AI transparency, and artwork AI transparency;
- deterministic Markdown and text document generation;
- bundled local Chromaprint screening of the authoritative release evidence and an optional explicitly triggered ACRCloud comparison;
- local, reproducible visible disclosure for AI-generated or AI-assisted artwork;
- native SHA-256 generation and verification;
- a finalization gate, PDF/A-2b completion certificates, evidence manifest, revisions, and optional revision-bound RFC-3161/external-timestamp addenda; and
- track-file-preserving discovery, conservative local indexing, and explicit adoption of existing track folders.

### Excluded from version 0.1

- cloud synchronization, remote backup, telemetry, tracking, or any required network connection;
- FastAPI, a backend service, PostgreSQL, Docker as a product dependency, or a local HTTP sidecar;
- accounts, login, multi-user workflows, or remote databases;
- Suno, OpenAI, Spotify, or distributor API integration;
- automatic upload, legal evaluation, legal advice, C2PA signing, blockchain, an application-created trusted timestamp, or independent validation of a timestamp's legal qualification; and
- automatic modification of existing or finalized evidence.

## Product principles

1. Do not lose user data or silently overwrite an existing file.
2. Do not invent missing facts or legal conclusions.
3. Show what is missing and ask only what is necessary.
4. Keep the final track folder understandable without the application or its SQLite index.
5. Treat finalized documentation as an immutable snapshot.
6. Keep filesystem, SQLite, hashing, artwork processing, and certificate operations behind narrow typed Rust commands.
7. Preserve the origin and derivation of evidence instead of trusting a role or filename alone.

## Architecture at a glance

```text
German desktop UI
Vite + TypeScript + HTML/CSS
              │
              │ typed Tauri 2 invoke calls
              ▼
Rust command and service boundary
├── WorkspaceService
├── TrackService
├── WorkflowService
├── EvidenceService
├── DocumentService
├── ArtworkService
├── HashService
├── CertificateService
└── PersistenceService
              │
              ├── <workspace>/.suno-doc/workspace.sqlite
              └── portable track folders and evidence
```

There is no product backend and no required network dependency. The normal workflow remains local; only user-started optional external timestamp and ACRCloud screening actions can contact a configured provider. TypeScript does not execute SQL, calculate authoritative hashes, or perform unrestricted filesystem operations. See [Product architecture](docs/def/product-architecture.md), [Pre-release audio screening](docs/def/pre-release-audio-screening.md), and [Persistence](docs/def/persistence.md) for the trust and data boundaries.

## Development quick start

Run commands from the repository root. Installing dependencies can require internet access; normal application use does not.

Native development requires a stable Rust compiler at or above the declared minimum supported Rust version (MSRV), Rust 1.88. The authoritative constraint is `rust-version = "1.88"` in `src-tauri/Cargo.toml`.

```sh
python tools/control.py doctor
python tools/control.py install
python tools/control.py tauri doctor
python tools/control.py tauri run --foreground
```

The browser-only Vite preview cannot perform the native workspace, evidence, artwork, SQLite, hash, or certificate operations. Use the Tauri development command for the complete product flow.

## User workflow

1. Create or open a local workspace.
2. Complete the minimal global artist, Suno, and artwork defaults.
3. Create a track as a single or assign it to a named album; scanned historical tracks default to singles.
4. Browse the collapsible album/single tree; reassign tracks or rename albums while the native layer moves the physical folders safely.
5. Follow the steps `01 Track` through `10 Finalize`.
6. Import real evidence with the native picker; the authoritative release evidence receives a local technical fingerprint automatically.
7. Generate documents and, when applicable, the visible AI artwork disclosure.
8. Optionally run the explicitly requested ACRCloud comparison in Step 07, then generate and verify `03_DOCUMENTATION/SHA256SUMS.txt`.
9. Finalize only after the application reports that every gate condition passes.
10. Preserve the generated certificate set, manifest, and both root-level technical PDFs with the track folder. After technical finalization, optionally attach external timestamp evidence to a stable anchor; a configured automatic RFC-3161 action always uses the finalized Evidence Manifest.

Start with [Getting started](docs/usr/getting-started.md). Before finalization, read [Finalizing a track](docs/usr/finalizing-a-track.md).

Step 10 creates `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` (English) and `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf` (German) directly in the track root. The local native renderer uses the same finalized structured snapshot as the Markdown certificate and evidence manifest; both PDFs occur in `06_CERTIFICATE/CERTIFICATE_SHA256.txt`. New format-6.1 PDFs and external-timestamp PDF addenda use PDF/A-2b with the complete DejaVu 2.37 Sans/Mono regular and bold font programs embedded under the DejaVu Fonts License, plus a CMYK FOGRA39 output intent.

Workflow 1.9 recognizes exactly delimited `made with suno studio` and `made with suno` metadata records in imported Suno WAV exports. A record becomes evidence-derived metadata only when it contains exactly one accepted marker, one valid RFC 3339 `created` timestamp, and one valid UUID `id`; this bounded structural check records file metadata and does not authenticate Suno or the provider. While a valid metadata date exists, it authoritatively fills and locks the final-generation date, production-end date, and optional download/export date. In Step 07, `No` to desktop-PC editing also derives and locks the last-editing date; `Yes` requires a freely selected date and the confirmed editing work. Manual fallbacks remain available when no valid metadata record exists. Section C of the certificate identifies the final generation date, Suno ID, project URL, model, plan at generation, metadata origin, and release/export hash comparison as separate facts.

Before finalization, SunoDM uses the bundled, pinned Chromaprint engine to record a real technical fingerprint of the managed authoritative `release_wav` evidence. The portable `03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json`, detached `LOCAL_FINGERPRINT.sha256`, and `AUDIO_SCREENING.md` bind the engine, algorithm, source Evidence ID/path/SHA-256, size, duration, and generated time; the full fingerprint remains only in the local fingerprint record and is never printed in the certificate, PDF, manifest, or UI. Step 07 can additionally send a deterministic set of bounded, non-overlapping audio samples—not the Chromaprint fingerprint—to ACRCloud after explicit Settings configuration and a user click. The configurable intensity can use the actual track duration or a fixed reference duration, while every run remains capped at 25 requests, 12 seconds per request, and 300 seconds of unique audio. This is optional, never runs in the background or during finalization, cannot block finalization, and records the plan, individual offsets/results, and provider-response hashes under the same hash-covered directory. A match/no-match is a technical provider result only, not a conclusion about authorship, ownership, permission, infringement, legality, or release clearance.

Suno Instrumental Mode, the intended vocal use, the actual final-audio result, and the content in Suno's Generation Text Field are independent answers. Content Classification is exactly one canonical value: `STRUCTURE_ONLY`, `VOCAL_LYRICS_ONLY`, `MIXED`, `EMPTY`, or `OTHER`; Vocal Intent is explicitly `VOCAL`, `INSTRUMENTAL`, or `UNSPECIFIED`. `EMPTY` is the N/A branch, every other classification requires the exact text and its `human`/`AI`/`mixed` source, and `OTHER` additionally requires a factual label. Intent is never inferred from text, classification, mode, or the final audio, and an intent/result difference is not a blocker. Historical arrays remain readable and are migrated only during an explicit workflow upgrade or new revision when their meaning is unambiguous; Vocal Intent is never migrated.

Audio AI questions and artwork AI questions are rendered separately. `NO` is an explicit user answer, `N/A` means a branch is logically inapplicable with a reason, and `NOT DOCUMENTED` means that sufficient information is absent. For commercial use with generative AI, an audio-disclosure status of `NOT DOCUMENTED` remains a blocker; an explicit `NO` may be recorded with a factual note and is not converted into a legal conclusion. Every AI artwork separately requires an explicit Artwork Disclosure `YES` or `NO`: `YES` requires text and the verified local disclosure lineage, while `NO` is visibly retained as deliberate non-application. Known provider names are offered consistently for new selections while historical and custom free text round-trips unchanged.

System verification compares SHA-256 values across all verified evidence and explicitly reports when the final release audio is byte-identical to the Suno export. It also reports a verified `human_edited_artwork`/`final_artwork` match as `BYTE-IDENTICAL / SHA-256 MATCH`; a mismatch is informational, not a finalization blocker. New artwork uses an ASCII-uppercase track stem with `_` separators (`My Track` → `MY_TRACK_AI_ORIGINAL.png`) and human-edited files use `_HUMAN_EDITED`; existing names and all finalized evidence remain unchanged. Artwork import timestamps record only import into SunoDM and do not establish the files' actual creation or editing sequence. Multiple adjacent global subscription receipts can jointly cover the production period, while any receipt that overlaps production or covers final generation can be attached. Coverage means only that documented date intervals match; it is not a license-validity or rights determination. Commercial finalization additionally requires an archived Terms PDF with document title, provider/source, and retrieval date; a source URL is recommended when known, while it, the effective date, applicable production period, and a factual note remain optional and are never fetched from the internet. A verified local Terms file and `Terms evidence not available: YES` are contradictory: the native update is rejected, workflow consistency blocks an imported legacy contradiction, and neither certificate renderer may emit it.

Finalized snapshots remain immutable: metadata extraction never backfills them in place, while an explicitly created revision may analyze the carried evidence. Optional external timestamp evidence is attached only after the phase-one finalization commit beneath `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/`. When automatic RFC-3161 attachment is configured, SunoDM fixes the anchor to the exact SHA-256 of `EVIDENCE_MANIFEST.json`, submits that digest with a fresh nonce and optional requested policy OID, and reports `VERIFIED` only when response structure/version/status, message imprint, nonce, policy contract, CMS signature, critical sole timestamping EKU, certificate validity at `genTime`, and the chain to explicitly configured TSA trust anchors all pass. A provider or verification failure is retained as timestamp status and never rolls back `DOCUMENTATION COMPLETE` or changes a phase-one byte. Manually attached/legacy evidence remains a hash-comparison record without automatic provider-identity, CMS, or chain claims; an initial OpenTimestamps proof remains `ATTACHED` until separately verified or upgraded.

The current UI separately derives an automatic-consistency presentation from those authoritative facts. `INFO` records non-blocking observations such as absent optional Suno metadata or byte-identical artwork; `WARNING` records an attached but not yet verified timestamp or another unresolved non-blocking finding; and an existing workflow contradiction remains `BLOCKING`. Informational findings retain `PASS`, warnings produce `PASS WITH WARNINGS`, and blockers produce `BLOCKED`. This display does not alter step status, finalization eligibility, certificate validity, or any immutable certificate byte.

Every sidecar format-v1 record is durably staged, verified, and parent-synchronized before SQLite registration, and only then published to its live certificate path; live-parent synchronization precedes any compensating database rollback. Startup publishes a valid registered pending stage, removes an unregistered abandoned stage, and rejects an unexpected unregistered live sidecar instead of adopting metadata. The immutable `TIMESTAMP_RECORD.json` records its certificate and finalization-snapshot binding, selected artifact, hashes, provider verification facts, publication-time integrity result, provenance, and pinned Markdown/PDF hashes; current `integrityVerified` and issues are computed presentation state. On load, SunoDM hashes the exact managed sidecar and referenced phase-one bytes without re-rendering them, verifies the exact registered JSON and file set, and reconstructs a positive RFC-3161 summary only when the complete current predicate still holds. Current and archived revision sidecars remain independently bound and reverified. Attachment does not alter the stamped anchor, create a cyclic self-hash, transfer to a later revision, or establish legal qualification.

Generated documents use template `1.11`; new finalizations write manifest schema `8` and certificate/PDF format `6.1`. SQLite schema `7` stores non-secret external-service configuration separately from the immutable finalized snapshot. Existing finalized artifacts remain byte-identical; there is no format, screening, timestamp, or font backfill. `PASS` means only that the configured documentation requirements for a step were satisfied. `DOCUMENTATION COMPLETE` means only that the configured documentation requirements for the finalized snapshot were completed. Neither status certifies authorship, ownership, non-infringement, legality, license validity, judicial weight, AI-law compliance, or governmental approval.

Archived Suno terms/rights files are selected once under `Einstellungen` together with their document title, provider/source, and retrieval date. Optional source URL, effective date, applicable production period, and a factual note add context without legal evaluation. SunoDM stores the local global record with its SHA-256 and metadata, then places a linked portable `global_copy` in every new or still editable project. Certificate summary and evidence-register detail refer to that same local Evidence ID, while `sourceGlobalEvidenceId` preserves the workspace-record link. Metadata edits propagate only to editable copies; finalized snapshots are never changed, so use a new revision before attaching newer or corrected terms.

## Local data model

The application deliberately uses two authorities:

| Data set | Source of truth | Reason |
| --- | --- | --- |
| Workspace configuration, track index, album/single placement, workflow status, evidence metadata, and UI state | Local SQLite database in `.suno-doc/` | Transactional local indexing and recovery support |
| Imported evidence, generated track documentation, hashes, manifests, certificates, and archived revisions | The track folder | Portable, human-readable verification without the application |

Global settings are copied into generated track documents as a dated snapshot. The documents never depend only on a mutable global setting. Stored paths are relative to their owning workspace or track root.

Album/single placement has two synchronized representations: typed SQLite metadata and the physical relative path. Singles live below `Singles/<track title>/`; album tracks live below `<album title>/<track title>/`. Reclassification and album renaming move the complete track root without changing any internal path or byte, so documents, hashes, and certificates remain valid. Managed tracks carry an excluded `.summary/track.json` identity marker so a renamed folder can be reconnected safely. See the [track library organization model](docs/def/track-library-model.md).

Every track evidence record has one explicit provenance value:

| Provenance | Meaning |
| --- | --- |
| `managed_copy` | Imported through the native track-evidence action and copied into the track |
| `global_copy` | Copied from registered workspace-global evidence into the portable track |
| `generated_disclosure` | Created locally by the versioned visible-disclosure generator |
| `indexed_legacy` | Discovered in an existing track folder and indexed conservatively rather than imported by the application |

The portable `EVIDENCE_MANIFEST.json` retains that provenance. A generated disclosure also retains its source evidence ID, generator version, and exact disclosure text. The native finalization gate uses this lineage and byte hashes; an ordinary import with the same role is not accepted as proof that the application generated the disclosure.

Post-finalization timestamp records use their own provenance statement. User-entered provider/type/reference fields, evidence-derived filename/hash facts, and system verification of the referenced local artifact hash remain visibly distinct.

For supported WAV evidence, the manifest also retains system-observed file/audio properties and bounded structured metadata. Valid Suno `created` and `id` values keep their evidence ID and SHA-256 origin, so a later replacement or removal can be reconciled without treating a filename or filesystem timestamp as proof.

Removing indexed legacy evidence is recoverable: the application moves a present file to `.archive/removals/<removal-id>/`, writes `removal.json`, and removes the index entry. Because the original path no longer exists, a later legacy scan does not re-index it. Historical content already under `06_CERTIFICATE/` is not treated as a failed finalization and is not moved unless the application-created `.archive/finalization-in-progress.json` marker proves that publication was interrupted.

The native boundary rejects traversal, absolute injected paths, and symbolic-link components observed during validation. Version 0.1 does not claim race-free containment against another process running as the same operating-system user that can modify the workspace and swap a path component between validation and use. Do not use a workspace writable by an untrusted concurrent process; descriptor-relative race hardening remains an open item in [ATP-0012](docs/atp/active/ATP-0012-filesystem-containment.md).

## Repository structure

```text
frontend/             German user interface and typed Tauri adapter
src-tauri/            Rust services, narrow commands, and native tests
workflows/            Versioned Suno workflow definition
docs/def/             Product architecture and domain definitions
docs/usr/             User task guides
docs/dev/             Development and migration guidance
docs/atp/active/      Executed and partially executed acceptance records
tools/control.py      Project lifecycle command
project-profile.toml  Generated desktop-local profile manifest
```

## Verification

These are the project verification commands; this document does not record an execution result.

```sh
python tools/control.py doctor
python tools/control.py tauri doctor
python tools/control.py test --suite all --report
python tools/control.py build web
python tools/control.py build desktop
python tools/control.py docs index --dry-run
python tools/control.py release check
```

Acceptance execution is recorded in the files under `docs/atp/active/`. [ATP-0016](docs/atp/active/ATP-0016-evidence-certificate-workflow-5.md) records the historical workflow-1.7 / certificate-5.0 release candidate; [ATP-0017](docs/atp/active/ATP-0017-pre-release-audio-screening.md) defines the required screening checks for the new format. The older [acceptance report](docs/dev/acceptance-report.md) and ATPs continue to describe only their identified earlier builds and certificate formats; their historical result rows are not reinterpreted.

## Detailed documentation

- [Product architecture](docs/def/product-architecture.md)
- [Track documentation model](docs/def/track-documentation-model.md)
- [Track library organization model](docs/def/track-library-model.md)
- [Persistence and recovery](docs/def/persistence.md)
- [Workflow model](docs/def/workflow-model.md)
- [Pre-release audio screening](docs/def/pre-release-audio-screening.md)
- [Getting started](docs/usr/getting-started.md)
- [Finalizing a track](docs/usr/finalizing-a-track.md)
- [Legacy track import](docs/dev/legacy-track-import.md)
- [ATP workflow](docs/atp/README.md)
- [Documentation standard](docs/README.md)

<!-- AUTO-GENERATED:docs-index START -->

## 📄 Files
- 📝 [Changelog](CHANGELOG.md)

# DOCS
- 📚 [Docs Home](docs/index.md)
- 📝 [<Document title>](docs/DOCUMENT-TEMPLATE.md)

## 📁 ATP
- 🗂️ [Overview](docs/atp/atp.md)
- 📝 [ATP-<ID>: <Acceptance title>](docs/atp/ATP-TEMPLATE.md)

## 📁 DEF
- 🗂️ [Overview](docs/def/def.md)
- 📝 [Application architecture](docs/def/architecture.md)
- 📝 [Runtime configuration — inherited template reference](docs/def/configuration.md)
- 📝 [Database feature — unavailable inherited reference](docs/def/database-feature.md)
- 📝 [Deployment architecture — unavailable inherited reference](docs/def/deployment-architecture.md)
- 📝 [Provider-neutral persistence architecture — inherited template reference](docs/def/persistence-architecture.md)
- 📝 [Local persistence and recovery](docs/def/persistence.md)
- 📝 [Pre-release audio screening](docs/def/pre-release-audio-screening.md)
- 📝 [Suno Documentation Manager product architecture](docs/def/product-architecture.md)
- 📝 [Project profiles — inherited template reference](docs/def/project-profiles.md)
- 📝 [Track documentation model](docs/def/track-documentation-model.md)
- 📝 [Track library organization model](docs/def/track-library-model.md)
- 📝 [Suno track workflow model](docs/def/workflow-model.md)

## 📁 DEV
- 🗂️ [Overview](docs/dev/dev.md)
- 📝 [Acceptance execution report — 2026-08-14](docs/dev/acceptance-report.md)
- 📝 [Legacy track import and managed-document adoption](docs/dev/legacy-track-import.md)
- 📝 [Upstream template final acceptance — historical reference](docs/dev/template-final-acceptance.md)

## 📁 Tools
- 🗂️ [Overview](docs/tools/tools.md)
- 📝 [Continuous integration — unavailable inherited reference](docs/tools/ci.md)
- 📝 [Container builds and local production simulation — unavailable inherited reference](docs/tools/container-builds.md)
- 📝 [Release and desktop packaging model](docs/tools/release-model.md)
- 📝 [Tooling guide](docs/tools/tooling.md)

## 📁 USR
- 🗂️ [Overview](docs/usr/usr.md)
- 📝 [Finalizing a track](docs/usr/finalizing-a-track.md)
- 📝 [Getting started with Suno Documentation Manager](docs/usr/getting-started.md)

<!-- AUTO-GENERATED:docs-index END -->

## Related documents

- [Documentation home](docs/index.md)
- [Desktop tooling](docs/tools/tauri/tauri.md)
- [Release model](docs/tools/release-model.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-20 | Added configurable deterministic ACRCloud multi-sample intensity with a 25-request / 300-second hard cap, response hashes, and certificate reporting; advanced templates to 1.11, manifest schema to 8, and certificate/PDF format to 6.1. | Project team |
| 2026-08-20 | Raised workflow to 1.9, templates to 1.10, manifest schema to 7, and certificate/PDF format to 6.0; added automatic cryptographically verified RFC-3161 manifest timestamps, PDF/A-2b with fully embedded DejaVu 2.37 fonts, canonical Content Classification and Vocal Intent values, explicit Artwork Disclosure decisions, artwork hash-identity/chronology reporting, and safe title-based artwork filenames. | Project team |
| 2026-08-18 | Added pre-release local Chromaprint screening and explicit optional ACRCloud screening documentation; advanced template to 1.9, manifest to 6, certificate/PDF to 5.1, and SQLite schema to 7. | Project team |
| 2026-08-17 | Documented final sidecar-v1 durability and recovery, byte-pinned current/archive verification, Terms contradiction prevention, and completed ATP-0016 acceptance evidence. | Project team |
| 2026-08-17 | Raised the evidence workflow to 1.7 and documented separated instrumental/vocal/Suno-field facts, complete final-generation and Terms context, audio/artwork AI assessments, precise status semantics, and revision-bound external-timestamp addenda; advanced template to 1.8, manifest to 5, certificate/PDF to 5.0, and SQLite schema to 5. | Project team |
| 2026-08-17 | Raised workflow to 1.6: WAV metadata now also supplies the optional download/export date and, when no desktop editing occurred, the locked last-editing date; adjacent subscription receipts are evaluated jointly. | Project team |
| 2026-08-17 | Raised workflow to 1.5: valid Suno metadata dates now authoritatively fill and lock Step 01 production end and Step 03 final generation; manual fallback remains available only without a metadata date. | Project team |
| 2026-08-17 | Documented workflow 1.4 evidence-derived Suno WAV metadata, conditional date automation, byte-identity verification, immutable revision handling, and artifact version updates. | Project team |
| 2026-08-14 | Clarified the collapsible album/single folder tree in the user workflow. | Project team |
| 2026-08-14 | Added the album/single library scope, workflow, persistence boundary, and detailed model link. | Project team |
| 2026-08-13 | Documented evidence provenance, disclosure lineage, recoverable legacy removal, marker-based recovery, the Rust MSRV, and the version 0.1 path-race limitation. | Project team |
| 2026-08-13 | Replaced the master-template overview with the Suno Documentation Manager product contract. | Project team |
