<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Track documentation model

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-14 |
| Audience | Product developers and documentation reviewers |
| Related ATP | [Track and document acceptance plans](../atp/active/active.md) |

## Purpose

This document defines the facts, evidence roles, generated documents, folder structure, and certificate artifacts that make one track documentation set portable and reviewable. It answers what is recorded and which representation is authoritative.

## Scope

### Included

- global defaults and immutable track snapshots;
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
| Track | Identity, dates, Suno facts, human-work declarations, release intent, and overall lifecycle status | Track snapshot documents plus index metadata |
| Workflow evaluation | Ten step states, applicability, reasons, missing requirements, and blocking deviations | Manifest and certificate for the finalized revision |
| Evidence record | Role, relative destination, media type, size, SHA-256, import time, and provenance note | Evidence file plus manifest entry |
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

The default profile does not request a birthday, private telephone number, private email address, Google account, or unrelated account data. A generated track document contains the values actually used as a snapshot. It never contains only a pointer to mutable global settings.

## Track facts

At minimum, the workflow can record the following confirmed facts:

- track title;
- production start and production end dates;
- Suno model and project URL;
- Suno plan at creation;
- final export date;
- lyrics source;
- whether external audio, own audio, or third-party samples were uploaded;
- whether human editing or post-export editing occurred;
- the specific confirmed human editing operations; and
- whether commercial use is intended.

An editing label such as `EQ preset`, `cuts`, `track editing`, `mastering/finalization`, or `post-export processing` appears only when the user confirms that operation. The generator does not add generic arrangement, mixing, or mastering claims by default.

## Conditional facts

The model stores a controlling answer separately from its dependent details. If `External audio uploaded?` is `No`, source, ownership, license, and uploaded-file questions are not required. If it is `Yes`, the dependent facts and evidence become applicable. The same rule applies to:

- own audio;
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
├── .archive/
│   ├── removals/
│   ├── recovery/
│   └── revisions/
├── 01_RELEASE/
├── 02_SUNO/
│   └── suno_project.txt
├── 03_DOCUMENTATION/
│   ├── AI_USAGE.md
│   ├── Lyrics.md
│   ├── README.md
│   ├── SHA256SUMS.txt
│   └── Styles.md
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
| Subscription or payment evidence (PDF, PNG/JPEG, TXT, or Markdown) | `04_LICENSES/` | Selected when its materialized coverage interval covers the track production period |
| Release WAV, MP3, or MP4 | `01_RELEASE/` | Release output; the configured final release role is mandatory |
| Release artwork | `01_RELEASE/` | Final release package artwork when applicable |
| AI artwork original | `05_ARTWORK/` | Required when an AI base image is declared |
| AI artwork edited | `05_ARTWORK/` | Required only when that production stage occurred |
| Human-edited artwork | `05_ARTWORK/` | Required only when that production stage occurred |
| Final artwork | `05_ARTWORK/` or release destination | Required when artwork is part of the release |
| Other evidence | Role-selected contained destination | Optional; must have a factual description |

`release_wav` and `final_artwork` are singular authoritative roles in version 0.1. To replace either asset, remove the current evidence through the managed action and then import the replacement; the app never chooses silently between competing final assets.

An import validates the type and role, calculates a safe destination, detects a collision, copies without deleting the source, calculates SHA-256, records size and metadata, and reevaluates the workflow. Manifest paths are relative to the track root.

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

For AI-generated or AI-assisted artwork, the default project transparency policy enables a visible local disclosure. The default text is `AI-assisted`, with reproducible bottom-right placement. The original is never overwritten. The output has `generated_disclosure` provenance, points through `derivedFromEvidenceId` to verified AI-original evidence, records `generatorVersion: local-disclosure-v1`, and retains the exact normalized disclosure text.

When disclosure is required, the gate checks that lineage and requires the final-artwork evidence to be byte-identical to the locally generated `AI_EDITED` disclosure output. Merely importing another image as `ai_artwork_edited`, asserting that disclosure occurred, or keeping a disclosed intermediate next to an unrelated final cover does not pass. `artwork_process.md` and `AI_USAGE.md` record the service, base image, human modifications, policy, whether disclosure was applied, disclosure text, and final output.

This is a project transparency policy. The product does not label it as a universally or legally mandatory watermark.

## Generated documents

The template version is recorded so that a document can be regenerated deterministically from the same normalized inputs. Generation combines the workspace snapshot, track facts, workflow results, and evidence metadata.

| Output | Minimum purpose |
| --- | --- |
| `02_SUNO/suno_project.txt` | Suno project URL and confirmed Suno production facts |
| `03_DOCUMENTATION/README.md` | Human-readable track documentation entry point |
| `03_DOCUMENTATION/AI_USAGE.md` | Confirmed AI systems, uses, and artwork disclosure facts |
| `03_DOCUMENTATION/Lyrics.md` | Lyrics source and confirmed lyrics content or reference |
| `03_DOCUMENTATION/Styles.md` | Confirmed style and production notes |
| `04_LICENSES/suno_account_and_license.md` | Snapshot of relevant account, plan, and selected subscription evidence facts |
| `04_LICENSES/openai_image_generation.md` | AI image service facts when applicable; the historical file name does not imply an API integration |
| `05_ARTWORK/artwork_process.md` | Artwork stages, human changes, content-check declarations, and disclosure result |

Generated prose states verifiable facts such as `External audio uploaded: No`. It does not claim that a track is guaranteed not to infringe copyright or that a license is legally sufficient.

## Integrity set

`03_DOCUMENTATION/SHA256SUMS.txt` lists root-relative paths in a format compatible with `sha256sum -c` where filenames permit. The set includes release files, Suno evidence, generated documentation, licenses, and artwork. It excludes:

- `.archive/`;
- `.summary/`;
- `03_DOCUMENTATION/SHA256SUMS.txt` itself;
- `06_CERTIFICATE/`; and
- workspace management data under the workspace root `.suno-doc/` (which is outside every track root).

A directory named `.suno-doc` inside a track subtree is ordinary track content and is included; the exclusion cannot be used to hide a release or evidence file.

After writing the list, the native service rereads every included file and verifies every digest. The integrity step passes only when the generated count and verified count match and no digest fails.

## Certificate artifacts

After the finalization gate passes, the product writes:

- `DOCUMENTATION_CERTIFICATE.md` with the certificate ID, track, artist, workflow ID and version, application version, finalization timestamp, mandatory-step result, justified N/A steps, evidence count, selected hashes, blocking-deviation result, and `DOCUMENTATION COMPLETE` status;
- `EVIDENCE_MANIFEST.json` with `schema_version`, track, artist, workflow, finalization, steps, evidence, hashes, and certificate objects; each verified evidence object includes provenance plus applicable source, coverage, and generated-disclosure lineage fields; and
- `CERTIFICATE_SHA256.txt` covering at least the main hash list, evidence manifest, and certificate document, but never itself.

The certificate ends with this meaning: it confirms completion of the configured documentation workflow and integrity checks, but is not governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.

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
| 2026-08-14 | Defined new-track dialog retention/dismissal behavior and clarified supported subscription-evidence formats. | Project team |
| 2026-08-13 | Added evidence provenance, portable disclosure lineage, and recoverable indexed-legacy removal. | Project team |
| 2026-08-13 | Defined the portable track documentation and certificate model. | Project team |
