# Suno Documentation Manager

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-14 |
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
- explicit evidence provenance for managed imports, global copies, locally generated disclosure outputs, and indexed legacy files;
- deterministic Markdown and text document generation;
- local, reproducible visible disclosure for AI-generated or AI-assisted artwork;
- native SHA-256 generation and verification;
- a finalization gate, completion certificate, evidence manifest, and revisions; and
- track-file-preserving discovery, conservative local indexing, and explicit adoption of existing track folders.

### Excluded from version 0.1

- cloud synchronization, remote backup, telemetry, tracking, or any required network connection;
- FastAPI, a backend service, PostgreSQL, Docker as a product dependency, or a local HTTP sidecar;
- accounts, login, multi-user workflows, or remote databases;
- Suno, OpenAI, Spotify, or distributor API integration;
- automatic upload, legal evaluation, legal advice, C2PA signing, blockchain, or qualified digital signatures; and
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

There is no product backend and no network dependency. TypeScript does not execute SQL, calculate authoritative hashes, or perform unrestricted filesystem operations. See [Product architecture](docs/def/product-architecture.md) and [Persistence](docs/def/persistence.md) for the trust and data boundaries.

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
6. Import real evidence with the native picker and resolve the displayed missing items.
7. Generate documents and, when applicable, the visible AI artwork disclosure.
8. Generate and verify `03_DOCUMENTATION/SHA256SUMS.txt`.
9. Finalize only after the application reports that every gate condition passes.
10. Preserve the generated certificate set, manifest, and root-level technical PDF with the track folder.

Start with [Getting started](docs/usr/getting-started.md). Before finalization, read [Finalizing a track](docs/usr/finalizing-a-track.md).

Step 10 now creates `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` directly in the track root. The local native renderer uses the same finalized structured snapshot as the Markdown certificate and evidence manifest; the PDF's SHA-256 is the fourth required entry in `06_CERTIFICATE/CERTIFICATE_SHA256.txt`.

Workflow 1.3 and certificate format 3.0 add explicit final Suno generation dates, instrumental and source-filename consistency checks, factual subscription coverage against the final-generation date, locally archived Suno terms/rights evidence, and optional external timestamp evidence. Project/version identifiers and a generation time are not requested. These are technical documentation facts only: the result does not certify authorship, ownership, non-infringement, legality, license validity, judicial weight, compliance, or governmental approval.

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

Acceptance execution and remaining manual checks are recorded in the files under `docs/atp/active/` and summarized in [the acceptance report](docs/dev/acceptance-report.md). An ATP moves to `completed/` only after every mandatory step passes.

## Detailed documentation

- [Product architecture](docs/def/product-architecture.md)
- [Track documentation model](docs/def/track-documentation-model.md)
- [Track library organization model](docs/def/track-library-model.md)
- [Persistence and recovery](docs/def/persistence.md)
- [Workflow model](docs/def/workflow-model.md)
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
| 2026-08-14 | Clarified the collapsible album/single folder tree in the user workflow. | Project team |
| 2026-08-14 | Added the album/single library scope, workflow, persistence boundary, and detailed model link. | Project team |
| 2026-08-13 | Documented evidence provenance, disclosure lineage, recoverable legacy removal, marker-based recovery, the Rust MSRV, and the version 0.1 path-race limitation. | Project team |
| 2026-08-13 | Replaced the master-template overview with the Suno Documentation Manager product contract. | Project team |
