<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Track documentation model

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-16 |
| Audience | Product developers and documentation reviewers |
| Related ATP | [Track and document acceptance plans](../atp/active/active.md) |

## Purpose

This document defines the facts, evidence roles, generated documents, folder structure, and certificate artifacts that make one track documentation set portable and reviewable. It answers what is recorded and which representation is authoritative.

## Scope

### Included

- global defaults, synchronized open-track snapshots, and immutable finalized snapshots;
- track metadata, evidence, documents, artwork stages, integrity data, and revisions;
- required folder and file names;
- factual-generation and privacy rules; and
- stable requirements mapped to acceptance plans.

### Excluded

- SQLite table definitions and migration mechanics;
- visual form layout;
- file-format transcoding of audio, video, or images other than the visible artwork disclosure;
- ownership or legal-compliance determinations; and
- remote evidence lookup.

## Core model

| Entity | Meaning | Authority after finalization |
| --- | --- | --- |
| Workspace profile | Reusable artist, Suno, subscription, commercial-intent, and artwork-policy defaults | Workspace SQLite database |
| Track | Identity, dates, concrete final Suno generation, human-work declarations, release intent, filename confirmations, and overall lifecycle status | Track snapshot documents plus index metadata |
| Workflow evaluation | Ten step states, applicability, reasons, missing requirements, and blocking deviations | Manifest and certificate for the finalized revision |
| Evidence record | Role, original and managed names, relative destination, media type, size, SHA-256, import time, provenance, lineage, and role-specific factual metadata | Evidence file plus manifest entry |
| Generated document | A versioned deterministic rendering of confirmed values and evidence references | Generated file in the track folder |
| Integrity snapshot | Root-relative file set and SHA-256 digest for each included file | `03_DOCUMENTATION/SHA256SUMS.txt` |
| Certificate snapshot | Final gate result, workflow/app versions, selected hashes, and disclaimer | Files below `06_CERTIFICATE/` |
| Revision | Archived previous certificate state and a new mutable working state | `.archive/revisions/<revision-id>/` and current track state |

The database can index these entities, but it is not the sole evidence of a finalized track.

## Workspace profile

The minimal reusable settings are:

| Field | Rule |
| --- | --- |
| Artist name | Required before a generated track snapshot can identify the artist |
| Suno profile name | Stored once and copied into relevant track documentation |
| Suno handle | Stored once and copied only where required |
| Suno plan | Default; each track records the plan actually used at creation |
| Suno subscription start date | Stored as a date, not inferred from evidence |
| Default commercial use intended | A default that the user confirms or changes for each track |
| Default AI image service | Used only when artwork is AI-generated or AI-assisted |
| AI artwork transparency policy | Defaults to `Always add visible AI disclosure` |

The default profile does not request a birthday, private telephone number, private email address, Google account, or unrelated account data. Saving the profile updates the embedded snapshot of every non-finalized track, marks its generated documents stale, and reevaluates its requirements. Opening an existing workspace performs the same reconciliation so tracks created or scanned by an older build receive the already saved global profile. Finalized and superseded tracks retain their historical snapshot. A generated track document contains the embedded values actually used; it never contains only a pointer to mutable global settings.

## Track facts

At minimum, the workflow can record the following confirmed facts:

- track title;
- production start and production end dates;
- Suno model, project URL, final-generation date, and download/export date are required user-confirmed local facts. Project/version ID, final generation ID, and final-generation time are no longer collected or emitted; compatibility fields remain readable in persisted legacy data and never block finalization. No value is fetched;
- Suno plan at creation, retained as unrestricted historical text;
- final export date;
- lyrics source;
- the lyrics text actually used when the track is not instrumental;
- the complete Suno style prompt;
- whether external audio, own audio, code-based generation, or third-party samples were used;
- the guided source category and rights basis for every applicable audio-source branch;
- the source-code or source-text evidence file, its generated WAV/MP3 output, the explicit post-processing answer, and any selected post-processing operations when code-based generation is confirmed;
- whether human editing or post-export editing occurred;
- the specific confirmed human editing operations;
- applicable human artwork process operations, editable process notes, and selected human changes to AI-assisted artwork; and
- whether commercial use is intended.

`instrumentalTrack` is an explicit answer. A positive answer is inconsistent with a non-instrumental lyrics source, retained lyrics text, or selected human work `Lyrics`; native finalization blocks until the user corrects the facts. It never silently changes them. A confirmed clean instrumental renders `Lyrics: N/A – instrumental track`.

The original local filenames captured at release-audio and Suno-export import are evidence-derived metadata. A normalized mismatch against the documented title is shown before finalization and needs explicit confirmation; the title is never derived from either filename.

Confirmed human-work labels are selected from the guided choices for arrangement, lyrics, timing/cuts, sound design, EQ, mixing, mastering, and loudness adjustment. Post-export work uses its own guided set for editing/cuts, arrangement, timing correction, sound design, EQ, mixing, mastering, loudness adjustment, noise reduction, and dynamics processing. Release notes likewise use guided release-version choices. A label appears only when the user selects it; the generator does not add generic arrangement, mixing, mastering, or release claims by default.

The localized UI label and the stored value are deliberately separate. Source, rights, and lyrics-source questions use clickable single-choice buttons, while activity lists use multi-choice buttons. New guided selections are persisted as stable English values even when the interface presents German labels. A recognized localized value from an older record is normalized on the next save. An unknown historical free-text value remains visible as a legacy choice for explicit reclassification and is not silently discarded.

## Conditional facts

The model stores a controlling answer separately from its dependent details. If `External audio uploaded?` is `No`, source, ownership, license, and uploaded-file questions are not required. If it is `Yes`, the dependent facts and evidence become applicable. The same rule applies to:

- own audio;
- code-based generation, which requires both a source-code/source-text evidence file and the generated WAV or MP3 only after an explicit positive answer; it then requires a post-processing `Yes`/`No`, and only `Yes` requires at least one operation;
- third-party samples;
- human lyrics and human editing;
- post-export processing;
- real people, real events, trademarks, and logos in artwork;
- AI-generated or AI-assisted artwork; and
- external license evidence.

A negative answer ends the branch. A positive content-check answer can require a factual note or evidence, but the product does not turn the answer into a legal conclusion.

## Portable track structure

```text
<track-folder>/
├── SunoDM_DOCUMENTATION_CERTIFICATE.pdf
├── .archive/
│   ├── removals/
│   ├── recovery/
│   └── revisions/
├── .summary/
│   └── track.json
├── 01_RELEASE/
├── 02_SUNO/
│   ├── Lyrics.md
│   ├── Style.md
│   └── suno_project.txt
├── 03_DOCUMENTATION/
│   ├── AI_USAGE.md
│   ├── README.md
│   └── SHA256SUMS.txt
├── 04_LICENSES/
│   ├── openai_image_generation.md
│   └── suno_account_and_license.md
├── 05_ARTWORK/
│   └── artwork_process.md
└── 06_CERTIFICATE/
    ├── DOCUMENTATION_CERTIFICATE.md
    ├── EVIDENCE_MANIFEST.json
    └── CERTIFICATE_SHA256.txt
```

Folder generation creates directories and managed text documents only when appropriate. It never creates empty audio, image, PDF, archive, or other fake evidence files.

## Evidence roles

| Role | Typical destination | Required condition or purpose |
| --- | --- | --- |
| Suno final export | `02_SUNO/` | Evidence of the selected Suno output when required by the workflow |
| Suno project ZIP | `02_SUNO/` | Optional project evidence |
| Suno screenshot | `02_SUNO/` | Optional factual evidence |
| Source code or source text (`.rb`, `.py`, `.txt`, `.md`, and other supported text-based formats) | `02_SUNO/` | Required only when code-based generation is confirmed |
| Code-generated audio (`.wav` or `.mp3`) | `02_SUNO/` | Required together with source-code evidence when code-based generation is confirmed |
| Subscription or payment evidence (PDF, PNG/JPEG, TXT, or Markdown) | `04_LICENSES/` | Selected when its materialized coverage interval covers the track production period |
| Final release audio (WAV, MP3, FLAC, M4A, AIFF, or OGG) | `01_RELEASE/` | Singular authoritative release output; the imported extension is preserved |
| Additional release MP3 or MP4 | `01_RELEASE/` | Optional additional release representation |
| Release artwork | `01_RELEASE/` | Final release package artwork when applicable |
| AI artwork original | `05_ARTWORK/` | Required when an AI base image is declared |
| AI artwork edited | `05_ARTWORK/` | Required only when that production stage occurred |
| Human-edited artwork | `05_ARTWORK/` | Required only when that production stage occurred |
| Final artwork | `05_ARTWORK/` | The authoritative JPG/PNG downloaded from Suno, or the required locally disclosed derivative |
| Archived Suno terms/rights | Global registration under `.suno-doc/global-evidence/`, portable copy under `04_LICENSES/` | One signature-checked local PDF without manual metadata; automatically assigned to new/editable projects |
| External timestamp evidence | `03_DOCUMENTATION/` | Optional local evidence plus provider/issuer, timestamp, referenced hash, and referenced artifact; no claim of legal qualification |
| Other evidence | Role-selected contained destination | Optional; must have a factual description |

`release_wav` and `final_artwork` are singular authoritative roles in version 0.1. To replace either asset, use the explicit upload control attached to the current evidence. The app reuses that evidence record, archives the previous managed bytes, and never chooses silently between competing final assets.

An import validates the type and role, calculates a safe destination, detects a collision, copies without deleting the source, calculates SHA-256 during the same streaming copy, records size and metadata, and reevaluates the workflow. It runs outside the webview thread. Routine loading avoids a repeated full SHA-256 read for evidence larger than 64 MiB; explicit evidence verification and integrity/finalization operations remain full cryptographic checks. Manifest paths are relative to the track root.

The authoritative release-audio copy is named `01_RELEASE/<safe track title>.<imported extension>`. The native filename sanitizer preserves readable title text while replacing filesystem-invalid characters and rejecting traversal or absolute paths. Import and title-change operations never overwrite an occupied target. A title change renames managed release evidence and its relative metadata only through the native Rust boundary, with rollback on failure. Finalized tracks remain immutable.

When an active, non-superseded track still has an exact managed legacy path such as `01_RELEASE/suno_final_export.wav`, loading may conservatively rename it to the title-based path and update the evidence record only if the managed provenance and role are unambiguous and the target is free. Indexed legacy, ambiguous, colliding, finalized, and superseded records remain unchanged.

The evidence UI exposes accepted file types for every role. Existing images and bounded text or source-code files can be previewed inside the app. Archive preview is metadata-only and never expands or reads an entire project ZIP into memory. The adjacent replacement action is distinct from preview so viewing evidence cannot accidentally open a file picker.

## Evidence provenance

An evidence role says what a file contributes to the workflow; provenance says how the application obtained it. The model stores both and does not derive provenance from a role, filename, or matching bytes.

| Provenance | Meaning | Additional fields |
| --- | --- | --- |
| `managed_copy` | Native evidence import copied a user-selected source into the track | None required |
| `global_copy` | Reusable workspace evidence was copied into the track | `sourceGlobalEvidenceId` and applicable coverage dates |
| `generated_disclosure` | The local native artwork generator created the file | `derivedFromEvidenceId`, `generatorVersion`, and `generatedDisclosureText` |
| `indexed_legacy` | A scan indexed a file already present in an existing track | Historical classification remains explicit even after the current bytes are verified |

The SQLite index retains these fields during work. Each verified evidence object written to `06_CERTIFICATE/EVIDENCE_MANIFEST.json` carries the same provenance and applicable lineage fields, so the finalized folder does not depend on SQLite for this distinction.

An explicit removal of indexed legacy evidence moves a present file to `.archive/removals/<removal-id>/` and writes `removal.json` with the original relative path and evidence metadata. This keeps the removal recoverable and prevents a later scan from re-indexing the old path. The removal archive is not part of the current integrity set.

## Artwork stages and naming

The supported naming convention is:

```text
<track-name>_AI_ORIGINAL.png
<track-name>_AI_EDITED.png
<track-name>_EDITED.png
<track-name>_FINAL.jpeg
```

`AI_ORIGINAL` is the unchanged AI output. `AI_EDITED` is a later AI-generated or AI-edited version. `EDITED` is a human-edited version. `FINAL` is the final artwork. Only stages that actually occurred are required.

For AI-generated or AI-assisted artwork, the default project transparency policy enables a visible local disclosure. The default text is `AI-assisted`, with reproducible bottom-right placement. The original is never overwritten. The output has `generated_disclosure` provenance, points through `derivedFromEvidenceId` to verified AI-original evidence, records `generatorVersion: local-disclosure-v1`, and retains the exact normalized disclosure text. When all three content checks—real person, authentic real event, and trademark/logo—are explicitly answered `No`, the AI Transparency step is deactivated and no disclosed derivative is required.

When disclosure is required, the gate checks that lineage and requires the final-artwork evidence to be byte-identical to the locally generated `AI_EDITED` disclosure output. Merely importing another image as `ai_artwork_edited`, asserting that disclosure occurred, or keeping a disclosed intermediate next to an unrelated final cover does not pass. `artwork_process.md` and `AI_USAGE.md` record the service, base image, human modifications, policy, whether disclosure was applied, disclosure text, and final output.

This is a project transparency policy. The product does not label it as a universally or legally mandatory watermark.

## Generated documents

Template version `1.5` is recorded so that a document can be regenerated deterministically from the same normalized inputs. Generation combines the track's current embedded profile snapshot, track facts, workflow results, and complete evidence metadata. Regeneration removes the previous managed `03_DOCUMENTATION/Lyrics.md` and `03_DOCUMENTATION/Styles.md`; an unmanaged file at either old path remains untouched.

Generated headings, explanatory prose, and guided-choice values are always English. German UI labels are mapped to their stable English values before rendering. An unknown legacy selection is represented by an English reclassification notice rather than copying potentially non-English unrestricted text into a generated choice field. User-authored factual content that must remain exact—such as lyrics, the Suno style prompt, a disclosure text, or an individually required factual note—is preserved verbatim and is not treated as generated prose.

| Output | Minimum purpose |
| --- | --- |
| `02_SUNO/suno_project.txt` | Suno project URL, confirmed production facts, code-generation and post-processing answers, selected operations, and applicable source-code plus generated-audio evidence paths |
| `02_SUNO/Lyrics.md` | Lyrics source and the exact used lyrics text when applicable |
| `02_SUNO/Style.md` | The complete style prompt entered in Suno |
| `03_DOCUMENTATION/README.md` | Human-readable track documentation entry point |
| `03_DOCUMENTATION/AI_USAGE.md` | Confirmed AI systems, code-audio post-processing facts, human changes to AI-assisted artwork, and disclosure facts |
| `04_LICENSES/suno_account_and_license.md` | Snapshot of relevant account, plan, and selected subscription evidence facts |
| `04_LICENSES/openai_image_generation.md` | AI image service facts when applicable; the historical file name does not imply an API integration |
| `05_ARTWORK/artwork_process.md` | Artwork stages, human changes, content-check declarations, and disclosure result |

Generated prose states verifiable facts such as `External audio uploaded: No`. It does not claim that a track is guaranteed not to infringe copyright or that a license is legally sufficient.

Generation publishes live phase, current-file, and completed-file counters to the invoking view while each managed output is written. This presentation state is not persisted and does not enter the templates, input fingerprint, or generated bytes; identical normalized inputs therefore remain byte-deterministic.

## Integrity set

`03_DOCUMENTATION/SHA256SUMS.txt` lists root-relative paths in a format compatible with `sha256sum -c` where filenames permit. The set includes release files, Suno evidence, generated documentation, licenses, and artwork. It excludes:

- `.archive/`;
- `.summary/`;
- `03_DOCUMENTATION/SHA256SUMS.txt` itself;
- the exact root path `SunoDM_DOCUMENTATION_CERTIFICATE.pdf`, which is anchored by the certificate hash set instead;
- `06_CERTIFICATE/`; and
- workspace management data under the workspace root `.suno-doc/` (which is outside every track root).

A directory named `.suno-doc` inside a track subtree is ordinary track content and is included; the exclusion cannot be used to hide a release or evidence file.

After writing the list, the native service rereads every included file and verifies every digest. The integrity step passes only when the generated count and verified count match and no digest fails.

Calculation and verification publish the actual processed byte and file counts from their bounded native read streams. Progress paths remain relative to the track root. These messages are presentation-only and cannot mark integrity as passed; the final native verification result remains authoritative.

## Certificate artifacts

After the finalization gate passes, the product writes:

- root-level `SunoDM_DOCUMENTATION_CERTIFICATE.pdf`, certificate format `3.0`, with A–J sections for identity, final Suno generation, all source branches, selected human contribution, AI usage, artwork checks, license/terms/coverage facts, the full evidence register, integrity anchors, and any locally recorded earlier revision-archive references;
- `DOCUMENTATION_CERTIFICATE.md` with the same factual scope, evidence register, origin labels, and explicit non-legal boundary;
- `EVIDENCE_MANIFEST.json` schema `2`, including the complete `documentedFacts` and `profileSnapshot`, origin-label definitions, system verification results, statement scope, and full evidence metadata/lineage; and
- `CERTIFICATE_SHA256.txt` covering the main hash list, evidence manifest, certificate document, and root-level PDF, but never itself.

All four outputs are staged and verified as one finalization transaction. The PDF hash is external to the PDF to avoid a circular self-hash. Its trailer identifiers are derived deterministically from the certificate ID, so identical normalized certificate snapshots serialize to identical bytes. Publication, rollback, crash recovery, and revision archival carry the root PDF together with the certificate directory, and an occupied root-PDF destination is never silently replaced.

The manifest, Markdown, and PDF share the sorted relative references of any earlier `.archive/revisions/<revision-id>/` entries that contain managed `revision.json` metadata. The certificate confirms recorded inputs, the finalized snapshot, local evidence and provenance, calculated hashes, and configured workflow checks. It expressly does not confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification. Facts are labeled where useful as `User-confirmed fact`, `Evidence-derived metadata`, or `System verification`.

After authoritative native commit, the UI may present a reusable certificate summary from the returned finalized `TrackDetail`. The summary does not create or reinterpret certificate data. It remains available in Finalize and the Certificate section only while the snapshot has a valid certificate ID, and links to the complete in-app certificate presentation.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-TRK-001` | Creating a track produces the required directory structure without fake evidence. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-TRK-002` | Conditional facts and evidence are required only when their controlling answers apply. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-TRK-003` | Interactions inside the new-track dialog retain the entered title, production-start date, and commercial-use choice; only a direct backdrop action or an explicit close/cancel control dismisses the dialog. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-DOC-001` | Versioned templates generate all required factual documents deterministically. | [ATP-0004](../atp/active/ATP-0004-document-generation.md) |
| `REQ-EVD-001` | Evidence import preserves the source, records metadata, provenance, and a hash, and never silently overwrites. | [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) |
| `REQ-ART-001` | Artwork stages follow the declared process and preserve the AI original. | [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) |
| `REQ-ART-002` | Applicable AI artwork receives a reproducible local visible disclosure whose source ID, generator version, exact text, and bytes remain traceable according to project policy. | [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md) |
| `REQ-HSH-001` | The correct included set is hashed and immediately verified; exclusions never enter the list. | [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md) |
| `REQ-CER-001` | A successful finalization writes a factual certificate, relative-path manifest, and certificate hashes. | [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) |

## Verification

Reviewers compare generated fixtures against the documented tree, ensure no placeholder media is created, and review deterministic output from identical normalized inputs. Planned commands, run from the repository root, are:

```sh
python tools/control.py test --suite tauri
python tools/control.py test --suite frontend
python tools/control.py docs index --dry-run
```

The authoritative acceptance records are [ATP-0002](../atp/active/ATP-0002-track-creation.md), [ATP-0004](../atp/active/ATP-0004-document-generation.md), [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md), [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md), [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md), and [ATP-0009](../atp/active/ATP-0009-certificate-generation.md). Their actual-result columns distinguish executed coverage from open checks.

## Risks and limitations

- Some legacy filenames or unsupported media types may not be automatically classified and require explicit user selection.
- `sha256sum` portability depends on filename encoding and platform conventions; the native verifier remains authoritative in the application.
- Hashes detect changes but do not establish who created a file or whether its contents are lawful.
- A user can intentionally remove portable files outside the application; the next scan or verification reports the resulting missing item or mismatch.

## Related documents

- [Product architecture](product-architecture.md)
- [Persistence and recovery](persistence.md)
- [Workflow model](workflow-model.md)
- [Finalizing a track](../usr/finalizing-a-track.md)
- [Legacy track import](../dev/legacy-track-import.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-16 | Added code-audio post-processing, human/AI-assisted artwork operations, factual content-check output, title-based release-audio naming, conservative legacy release migration, and template version 1.4. | Project team |
| 2026-08-16 | Replaced Source/right and lyrics-source dropdowns with mutually exclusive guided buttons without changing stored canonical values. | Project team |
| 2026-08-16 | Required the generated WAV/MP3 alongside source code for code-based generation and advanced generated documents to template version 1.3. | Project team |
| 2026-08-15 | Defined the post-commit reusable certificate summary as a presentation of authoritative finalized track data. | Project team |
| 2026-08-16 | Added final-generation identity, instrumental and filename consistency, subscription-date coverage, local terms/timestamp evidence, document template 1.5, manifest schema 2, and certificate format 3.0. | Project team |
| 2026-08-15 | Defined non-persistent live progress for deterministic document writes and native integrity reads. | Project team |
| 2026-08-15 | Added guided Source classifications, conditional source-code evidence, English choice rendering, legacy reclassification, and template version 1.2. | Project team |
| 2026-08-14 | Defined new-track dialog retention/dismissal behavior and clarified supported subscription-evidence formats. | Project team |
| 2026-08-13 | Added evidence provenance, portable disclosure lineage, and recoverable indexed-legacy removal. | Project team |
| 2026-08-13 | Defined the portable track documentation and certificate model. | Project team |
