<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Track documentation model

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-20 |
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
- automatic remote evidence lookup.

## Core model

| Entity | Meaning | Authority after finalization |
| --- | --- | --- |
| Workspace profile | Reusable artist, Suno, subscription, commercial-intent, and artwork-policy defaults | Workspace SQLite database |
| Track | Identity, dates, concrete final Suno generation, human-work declarations, release intent, filename confirmations, and overall lifecycle status | Track snapshot documents plus index metadata |
| Workflow evaluation | Ten step states, applicability, reasons, missing requirements, and blocking deviations | Manifest and certificate for the finalized revision |
| Evidence record | Role, original and managed names, relative destination, media type, technical audio properties, structured embedded metadata, size, SHA-256, import time, provenance, lineage, and role-specific factual metadata | Evidence file plus manifest entry |
| Generated document | A versioned deterministic rendering of confirmed values and evidence references | Generated file in the track folder |
| Audio-screening state | Current local fingerprint binding and optional explicit provider-result summary for the authoritative release source | Portable screening artifacts plus mutable track state; raw fingerprints/responses remain outside certificate summaries |
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
| Suno plan | Default; each track records the plan actually used for the final generation |
| Suno subscription start date | Stored as a date, not inferred from evidence |
| Default commercial use intended | A default that the user confirms or changes for each track |
| Default AI image service | Used only when artwork is AI-generated or AI-assisted |
| AI artwork transparency policy | Defaults to `Always add visible AI disclosure` |

The default profile does not request a birthday, private telephone number, private email address, Google account, or unrelated account data. Saving the profile updates the embedded snapshot of every non-finalized track, marks its generated documents stale, and reevaluates its requirements. Opening an existing workspace performs the same reconciliation so tracks created or scanned by an older build receive the already saved global profile. Finalized and superseded tracks retain their historical snapshot. A generated track document contains the embedded values actually used; it never contains only a pointer to mutable global settings.

## Track facts

At minimum, the workflow can record the following confirmed facts:

- track title;
- production start and production end dates; production end can be evidence-derived only under the rule below;
- Suno model, project URL, optional project/version ID, final-generation ID/Suno ID, final-generation date, optional download/export date, and last-editing date. The three dates can be evidence-derived under the rules below. A valid ID observed in the Suno export remains evidence-derived; compatible user-entered ID fields are shown only when present and are never conflated. No value is fetched from Suno;
- Suno plan at generation, retained as unrestricted user-confirmed text;
- whether the WAV was edited again on the desktop PC and the resulting last-editing date;
- the current technical local screening record for authoritative release evidence: Chromaprint engine/version, algorithm, source Evidence ID/path/SHA-256, size, measured duration, generation time, and artifact path/hash; and
- when the user explicitly requests it, a concise ACRCloud provider result bound to the same release source, bounded sample timings, response artifact path/hash, and provider-supplied match fields;
- whether Suno Instrumental Mode was selected;
- the explicitly selected Vocal Intent and whether sung or spoken vocals are actually present in the final audio;
- exactly one Content Classification for the Suno Generation Text Field and, when non-empty, its human/AI/mixed source and exact field text;
- the complete Suno style prompt;
- whether external audio, own audio, code-based generation, or third-party samples were used;
- the guided source category and rights basis for every applicable audio-source branch;
- the source-code or source-text evidence file, its generated WAV/MP3 output, the explicit post-processing answer, and any selected post-processing operations when code-based generation is confirmed;
- whether human editing or post-export editing occurred;
- the specific confirmed human editing operations;
- applicable human artwork process operations, editable process notes, and selected human changes to AI-assisted artwork; and
- whether commercial use is intended.

Suno Instrumental Mode, Vocal Intent, the final audio result, and Suno Generation Text Field content are separate facts. Content Classification stores exactly one of `STRUCTURE_ONLY`, `VOCAL_LYRICS_ONLY`, `MIXED`, `EMPTY`, or `OTHER`. Vocal Intent stores exactly one consciously selected value: `VOCAL`, `INSTRUMENTAL`, or `UNSPECIFIED`. `EMPTY` clears and makes source, exact text, and Other label inapplicable. Every other classification requires the exact text and a separately recorded `human`, `AI`, or `mixed` source; `OTHER` also requires its factual label. Vocal Lyrics plus structure instructions must be recorded as the single value `MIXED`.

No cross-field blocker compares Vocal Intent or Suno Instrumental Mode with the final audio result. `VOCAL` with no vocals in the final audio and `INSTRUMENTAL` with vocals in the final audio are both valid factual records. Vocal Intent is never inferred from text, Content Classification, Suno Instrumental Mode, or audio analysis. Generated documents use the neutral `Suno Generation Text Field` heading and print the canonical tokens directly.

Historical `lyricsSource`, `lyricsText`, the old field-content Boolean, and the old multi-value content array remain readable through explicitly labeled legacy compatibility fields. Ordinary reads and saves never reinterpret them. Only an explicit workflow upgrade or new revision may migrate an unambiguous old classification: false to `EMPTY`, only vocal to `VOCAL_LYRICS_ONLY`, only non-vocal instructions to `STRUCTURE_ONLY`, vocal plus non-vocal instructions to `MIXED`, and only Other to `OTHER`. Historical Mixed or any Other combination remains unclassified. Vocal Intent is never migrated.

The original local filenames captured at release-audio and Suno-export import are evidence-derived metadata. A normalized mismatch against the documented title is shown before finalization and needs explicit confirmation; the title is never derived from either filename.

Confirmed human-work labels are selected from the guided choices for arrangement, lyrics, timing/cuts, sound design, EQ, mixing, mastering, and loudness adjustment. Post-export work uses its own guided set for editing/cuts, arrangement, timing correction, sound design, EQ, mixing, mastering, loudness adjustment, noise reduction, and dynamics processing. Release notes likewise use guided release-version choices. A label appears only when the user selects it; the generator does not add generic arrangement, mixing, mastering, or release claims by default.

The localized UI label and the stored value are deliberately separate. Source, rights, and lyrics-source questions use clickable single-choice buttons, while activity lists use multi-choice buttons. New guided selections are persisted as stable English values even when the interface presents German labels. A recognized localized value from an older record is normalized on the next save. An unknown historical free-text value remains visible as a legacy choice for explicit reclassification and is not silently discarded.

## Conditional facts

The model stores a controlling answer separately from its dependent details. If `External audio uploaded?` is `No`, source, ownership, license, and uploaded-file questions are not required. If it is `Yes`, the dependent facts and evidence become applicable. The same rule applies to:

- own audio;
- code-based generation, which requires both a source-code/source-text evidence file and the generated WAV or MP3 only after an explicit positive answer; it then requires a post-processing `Yes`/`No`, and only `Yes` requires at least one operation;
- third-party samples;
- the scalar Suno Generation Text Field classification and human editing;
- post-export processing;
- real people, real events, trademarks, and logos in artwork;
- generative AI use in audio, AI-generated or AI-assisted artwork; and
- external license evidence.

A negative answer ends the branch. A positive content-check answer can require a factual note or evidence, but the product does not turn the answer into a legal conclusion.

## Portable track structure

```text
<track-folder>/
├── SunoDM_DOCUMENTATION_CERTIFICATE.pdf
├── SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf
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
│   ├── AUDIO_SCREENING/
│   │   ├── LOCAL_FINGERPRINT.json
│   │   ├── LOCAL_FINGERPRINT.sha256
│   │   ├── AUDIO_SCREENING.md
│   │   ├── ACRCLOUD_SCREENING.json        # optional, explicit provider action only
│   │   └── ACRCLOUD_RESPONSE.json          # optional, explicit provider action only
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
    ├── CERTIFICATE_SHA256.txt
    └── EXTERNAL_TIMESTAMPS/              # optional, post-finalization
        └── <timestamp-record-id>/
            ├── TIMESTAMP_RECORD.json      # immutable sidecar format v1
            ├── TIMESTAMP_EVIDENCE.<ext>
            ├── PROVIDER_RESPONSE.<ext>    # optional exact raw response
            ├── EXTERNAL_TIMESTAMP_ADDENDUM.md
            ├── EXTERNAL_TIMESTAMP_ADDENDUM.pdf
            └── TIMESTAMP_RECORD_SHA256.txt
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
| Subscription or payment evidence (PDF) | `04_LICENSES/` | Selected when its interval overlaps production or covers final generation; adjacent selected intervals jointly satisfy the production-period gate |
| Final release audio (WAV, MP3, FLAC, M4A, AIFF, or OGG) | `01_RELEASE/` | Singular authoritative release output; the imported extension is preserved and triggers local screening while editable |
| Additional release MP3 or MP4 | `01_RELEASE/` | Optional additional release representation |
| Release artwork | `01_RELEASE/` | Final release package artwork when applicable |
| AI artwork original | `05_ARTWORK/` | Required when an AI base image is declared |
| AI artwork edited | `05_ARTWORK/` | Required only when that production stage occurred |
| Human-edited artwork | `05_ARTWORK/` | Required only when that production stage occurred |
| Final artwork | `05_ARTWORK/` | The authoritative JPG/PNG downloaded from Suno, or the required locally disclosed derivative |
| Archived Suno terms/rights | Global registration under `.suno-doc/global-evidence/`, portable copy under `04_LICENSES/` | One signature-checked local PDF with document title, provider/source, and retrieval date; source URL is recommended when known, while it, effective date, applicable production period, and factual note remain optional; automatically assigned to new/editable projects |
| External timestamp evidence | `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/<timestamp-record-id>/` | Optional phase-two attachment after technical finalization; automatic RFC-3161 uses only the exact finalized Evidence Manifest, while manual/legacy records may bind another permitted stable artifact; no legal-qualification claim |
| Other evidence | Role-selected contained destination | Optional; must have a factual description |

`suno_final_export`, `release_wav`, and `final_artwork` are singular authoritative roles in version 0.1. To replace an asset, use the explicit upload control attached to the current evidence. The app reuses that evidence record, archives the previous managed bytes, and never chooses silently between competing final assets.

An ordinary pre-finalization import validates the type and role, calculates a safe destination, detects a collision, copies without deleting the source, calculates SHA-256 during the same streaming copy, records size and metadata, and reevaluates the workflow. It runs outside the webview thread. Routine loading avoids a repeated full SHA-256 read for evidence larger than 64 MiB; explicit evidence verification and integrity/finalization operations remain full cryptographic checks. Manifest paths are relative to the track root. The external-timestamp role is excluded from this generic path because its referenced anchor does not exist until technical finalization.

## Evidence-derived WAV metadata

For a WAV import, the native reader walks bounded RIFF chunks and reads structured text metadata plus technical format facts without scanning the audio payload as text. Ordinary WAV files remain valid evidence. Only the complete case-insensitive segments `made with suno studio` and `made with suno` are accepted as markers. A usable record requires exactly one accepted marker, one RFC 3339 `created` value, and one UUID `id`; duplicate, mixed, malformed, incidental, or unsafe records produce no derived Suno facts. The evidence record preserves the exact raw structured value, parsed timestamp, calendar date, normalized ID, audio format, channel count, sample rate, duration, and bit depth when present. These checks describe local file structure and do not authenticate the provider.

These observations have the origin `Evidence-derived metadata` and retain the source evidence ID and SHA-256. They are not inferred from the filename, import time, creation time, modification time, or other filesystem attributes. Malformed, incomplete, oversized, or unrelated metadata is ignored safely and does not become a track fact.

For an editable track, a valid Suno final export authoritatively fills the final-generation date in Step 03, production-end date in Step 01, and optional download/export date in Step 03 from the embedded `created` calendar date. These inputs are read-only while that metadata date exists. In Step 07, a confirmed `No` to further desktop-PC editing also derives and locks the last-editing date; `Yes` keeps it user-confirmed. Manual fallbacks are accepted when no valid metadata date exists.

Replacement updates all applicable values to the new evidence date. Removal clears the system-owned values and re-enables manual fallbacks. Finalized and superseded snapshots are never analyzed or backfilled during normal loading, including after the accepted-marker list changes. Creating a revision first produces mutable working state in which the hash-matching carried WAV evidence can be analyzed while the archived certificate, manifest, and timestamp sidecars remain immutable.

The authoritative release-audio copy is named `01_RELEASE/<safe track title>.<imported extension>`. The native filename sanitizer preserves readable title text while replacing filesystem-invalid characters and rejecting traversal or absolute paths. Import and title-change operations never overwrite an occupied target. A title change renames managed release evidence and its relative metadata only through the native Rust boundary, with rollback on failure. Finalized tracks remain immutable.

## Pre-release audio screening

Audio screening is not an evidence role and does not create a substitute audio identity from SHA-256. For an editable track, import or replacement of the singular authoritative `release_wav` runs the packaged target-verified Chromaprint `fpcalc` engine against the managed source. The durable local record carries the actual fingerprint, but only `LOCAL_FINGERPRINT.json` contains that value. `AUDIO_SCREENING.md`, `03_DOCUMENTATION/README.md`, the manifest, Markdown certificate, and PDF show a concise status/source/artifact summary only.

An optional ACRCloud request is available only after Settings stores a non-secret workspace configuration and private write-only credentials. It is initiated in Step 07 by the user, uses a bounded audio sample rather than a Chromaprint fingerprint, and archives a raw provider response only at `ACRCLOUD_RESPONSE.json`. The portable external summary includes source linkage, sample timing, request count, response path/hash, and at most five concise provider matches. It never includes a credential, request signature, raw fingerprint, or unrestricted provider output.

Local `FINGERPRINT GENERATED` is required for the current release source before a new finalization. `NOT RUN`, `STALE`, engine availability, unsupported-format, configuration, provider, and processing states are not positive assertions. An optional provider match/no-match never blocks finalization and never states ownership, permission, infringement, legality, copyright safety, or release clearance. A replacement invalidates source linkage; a revision keeps the prior screening directory with the archived snapshot and begins a new state rather than transferring an external result.

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

## AI transparency assessments

AI transparency is a factual questionnaire with separate Audio and Artwork representations. It is not an AI Act, copyright, personality-right, or disclosure-law assessment.

The Audio assessment first requires an explicit `Generative AI used` answer. A positive answer records the AI system and one tri-state answer for each of these facts:

- AI-assisted audio elements;
- AI-generated audio elements;
- intentional imitation of a real person's voice;
- intentional representation of a real person's identity;
- representation of a real event as an authentic recording;
- presentation of a real location, institution, or event as an authentic AI recording; and
- audio disclosure applied.

The tri-state values are `YES`, `NO`, and `NOT DOCUMENTED`. `NO` is a deliberate user-confirmed negative answer. `NOT DOCUMENTED` records that sufficient information is absent; it is never silently converted to `NO`. If disclosure is `YES`, at least one location and the exact disclosure text are required. If disclosure is `NO`, the user may retain a factual reason. For a commercially intended track that uses generative AI, `Disclosure applied = NOT DOCUMENTED` blocks finalization; a deliberate `NO` remains a documented answer and is not evaluated for legal sufficiency.

The system may summarize whether the audio questionnaire is complete and whether a potential deepfake-related indicator was recorded. It must not output `No deepfake`, `AI Act compliant`, `Disclosure legally unnecessary`, or an equivalent legal conclusion. Known service names such as `Suno` and `ChatGPT / OpenAI` can be offered as consistent new-entry suggestions, while historical, future, and custom free text remains unchanged.

The Artwork assessment separately retains the artwork origin, service, real-person/event and trademark/logo answers, human modifications, artwork disclosure policy, result, text, and generated-artifact lineage. Every AI-generated or AI-assisted artwork requires an explicit `YES` or `NO` disclosure decision, independently of the configured policy and the three content-check answers. `YES` requires non-empty disclosure text and a verified locally generated artifact with the recorded lineage. `NO` is stored and rendered as a deliberate non-application; an absent decision remains incomplete. Audio answers do not satisfy Artwork questions and Artwork answers do not satisfy Audio questions.

## Artwork stages and naming

The supported naming convention is:

```text
<TRACK_STEM>_AI_ORIGINAL.png
<TRACK_STEM>_AI_EDITED.png
<TRACK_STEM>_HUMAN_EDITED.png
<TRACK_STEM>_FINAL.jpeg
```

`TRACK_STEM` is derived for new files as ASCII uppercase with `_` separators (`My Track` becomes `MY_TRACK`). `AI_ORIGINAL` is the unchanged AI output. `AI_EDITED` is a later AI-generated or AI-edited version. `HUMAN_EDITED` is a human-edited version. `FINAL` is the final artwork. Only stages that actually occurred are required. The convention applies only to newly imported, replaced, or locally generated artwork. Existing mixed-case, hyphenated, and `_EDITED` paths remain readable for legacy role detection, and finalized or superseded evidence is never renamed or migrated in place.

For AI-generated or AI-assisted artwork, the default project transparency policy enables a visible local disclosure. The default text is `AI-assisted`, with reproducible bottom-right placement. The original is never overwritten. The output has `generated_disclosure` provenance, points through `derivedFromEvidenceId` to verified AI-original evidence, records `generatorVersion: local-disclosure-v1`, and retains the exact normalized disclosure text. Three explicit negative artwork content-check answers close their dependent note branches; they do not remove the separate Artwork disclosure decision or the Audio assessment.

When disclosure is `YES`, the gate checks that lineage and requires the final-artwork evidence to be byte-identical to the locally generated `AI_EDITED` disclosure output. Merely importing another image as `ai_artwork_edited`, asserting that disclosure occurred, or keeping a disclosed intermediate next to an unrelated final cover does not pass. Independently, verified `human_edited_artwork` evidence is compared with the single verified `final_artwork` by SHA-256. A match is documented as `BYTE-IDENTICAL / SHA-256 MATCH`; a mismatch is informational and never a blocker. `artwork_process.md`, `AI_USAGE.md`, and `openai_image_generation.md` record the explicit disclosure decision and the limitation that import timestamps prove only import into SunoDM, not the actual creation or editing chronology.

This is a project transparency policy. The product does not label it as a universally or legally mandatory watermark.

## Generated documents

Template version `1.11` is recorded so that a document can be regenerated deterministically from the same normalized inputs. Generation combines the track's current embedded profile snapshot, track facts, workflow results, complete evidence metadata, and current audio-screening state including multi-sample coverage; fact origins and automation results are additionally retained in the manifest and certificate snapshot. The internal freshness digest may include the full local fingerprint, but managed prose never does. Regeneration removes the previous managed `03_DOCUMENTATION/Lyrics.md` and `03_DOCUMENTATION/Styles.md`; an unmanaged file at either old path remains untouched.

Generated headings, explanatory prose, and guided-choice values are always English. German UI labels are mapped to their stable English values before rendering. An unknown legacy selection is represented by an English reclassification notice rather than copying potentially non-English unrestricted text into a generated choice field. User-authored factual content that must remain exact—such as lyrics, the Suno style prompt, a disclosure text, or an individually required factual note—is preserved verbatim and is not treated as generated prose.

| Output | Minimum purpose |
| --- | --- |
| `02_SUNO/suno_project.txt` | Final-generation date, IDs when present, project URL, metadata detection/origin, model, plan at generation, download/export date, code-generation and post-processing answers, selected operations, and applicable source-code plus generated-audio evidence paths |
| `02_SUNO/Lyrics.md` | Canonical Content Classification and Vocal Intent tokens, independent final-audio result, source, and exact field text under a neutral `Suno Generation Text Field` heading; unclassified legacy values remain labeled legacy/`NOT DOCUMENTED` |
| `02_SUNO/Style.md` | The complete style prompt entered in Suno |
| `03_DOCUMENTATION/README.md` | Human-readable track documentation entry point, including a concise pre-release audio-screening status/source/artifact summary without a raw fingerprint or secret |
| `03_DOCUMENTATION/AI_USAGE.md` | Separate Audio and Artwork assessments, exact `YES`/`NO`/`NOT DOCUMENTED` answers, confirmed AI systems, code-audio post-processing facts, human artwork changes, and disclosure facts |
| `04_LICENSES/suno_account_and_license.md` | Snapshot of the plan at generation, technical subscription-date coverage, and selected Terms evidence with complete stored metadata |
| `04_LICENSES/openai_image_generation.md` | AI image service facts when applicable; the historical file name does not imply an API integration |
| `05_ARTWORK/artwork_process.md` | Artwork stages, human changes, content-check declarations, and disclosure result |

Generated prose states verifiable facts such as `External audio uploaded: No`. It does not claim that a track is guaranteed not to infringe copyright or that a license is legally sufficient.

Generation publishes live phase, current-file, and completed-file counters to the invoking view while each managed output is written. This presentation state is not persisted and does not enter the templates, input fingerprint, or generated bytes; identical normalized inputs therefore remain byte-deterministic.

## Integrity set

`03_DOCUMENTATION/SHA256SUMS.txt` lists root-relative paths in a format compatible with `sha256sum -c` where filenames permit. The set includes release files, Suno evidence, generated documentation, licenses, and artwork. It excludes:

- `.archive/`;
- `.summary/`;
- `03_DOCUMENTATION/SHA256SUMS.txt` itself;
- the exact root paths `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` and `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf`, which are anchored by the certificate hash set instead;
- `06_CERTIFICATE/`; and
- workspace management data under the workspace root `.suno-doc/` (which is outside every track root).

A directory named `.suno-doc` inside a track subtree is ordinary track content and is included; the exclusion cannot be used to hide a release or evidence file.

After writing the list, the native service rereads every included file and verifies every digest. The integrity step passes only when the generated count and verified count match and no digest fails.

The same verified SHA-256 values drive byte-identity reporting. System verification lists every pair of verified evidence records with identical bytes, regardless of role, and separately states whether the authoritative final release audio is byte-identical to the Suno final export. The result is hash-based; equal names, dates, sizes, or metadata alone never establish identity.

Calculation and verification publish the actual processed byte and file counts from their bounded native read streams. Progress paths remain relative to the track root. These messages are presentation-only and cannot mark integrity as passed; the final native verification result remains authoritative.

## Certificate artifacts

After the finalization gate passes, the product writes:

- root-level `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` (English) and `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf` (German), certificate format `6.0`, as deterministic PDF/A-2b documents with A–L sections for identity; the separated instrumental, vocal, and Suno-field facts; every source branch; selected human contribution; separate Audio and Artwork AI assessments; license/Terms/technical-coverage facts; the full evidence register; integrity anchors; K.2 audio-screening summary; and locally recorded earlier revision-archive references;
- `DOCUMENTATION_CERTIFICATE.md`, certificate format `6.0`, with the same factual scope, evidence register, origin labels, status meanings, K.2 screening summary, and explicit non-legal boundary;
- `EVIDENCE_MANIFEST.json` schema `7`, including the complete `documented_facts` and `profile_snapshot`, fact-origin and answer-state definitions, separated lyrics and AI assessment values, Suno detection summary, generic byte-identical pairs, automatic role relationships, consistency results, a sanitized `audioScreening` section, the certificate PDF archive/font metadata, statement scope, and full evidence metadata/lineage; and
- `CERTIFICATE_SHA256.txt` covering the main hash list, evidence manifest, certificate document, and both root-level PDFs, but never itself.

All five outputs are staged and verified as one finalization transaction. The PDF hashes are external to the PDFs to avoid circular self-hashes. Their trailer identifiers are derived deterministically from the certificate ID, so identical normalized certificate snapshots serialize to identical bytes. Publication, rollback, crash recovery, and revision archival carry both root PDFs together with the certificate directory, and occupied root-PDF destinations are never silently replaced.
The certificate/manifest screening summary excludes the full fingerprint, raw ACRCloud response, request signature, access key, and access secret. It is a technical comparison record only and does not state a rights or legal conclusion.

The PDF renderer paginates long sections and wraps long URLs, UUIDs, paths, labels, and SHA-256 values without truncating their factual content. SHA-256 labels remain intact, full digests remain readable, and every page carries the Certificate ID plus `Seite X / Y`. The same normalized certificate snapshot produces the same factual content and deterministic bytes; transient UI state and random presentation data do not enter the renderer.

Current certificate PDFs and timestamp addendum PDFs identify themselves as PDF/A-2b in XMP, use PDF 1.7 without encryption, and carry the CMYK `Coated FOGRA39 (ISO 12647-2:2004)` output intent. The renderer embeds the complete font programs rather than subsets. The bundled font identity is fixed as follows and is also emitted in manifest schema 8 under `certificate.pdf_archive` with archive format, embedding mode `full`, font version `DejaVu 2.37`, license `DejaVu Fonts License`, and output intent:

| Embedded font | SHA-256 |
| --- | --- |
| `DejaVuSans` | `7da195a74c55bef988d0d48f9508bd5d849425c1770dba5d7bfc6ce9ed848954` |
| `DejaVuSans-Bold` | `e6476c1b80502924294eed40894c5b18e06c181444ca953e5334262df9c27724` |
| `DejaVuSansMono` | `b4a6c3e4faab8773f4ff761d56451646409f29abedd68f05d38c2df667d3c582` |
| `DejaVuSansMono-Bold` | `bce60f1b4421acd9ea51ba6623d7024ecbe6817a953e3654df62a5e6bdf8f769` |

The bundled license text is retained at [`src-tauri/assets/fonts/LICENSE-DejaVu.txt`](../../src-tauri/assets/fonts/LICENSE-DejaVu.txt). Current-format verification requires XMP PDF/A identification, the expected output intent, embedded non-subset font programs with ToUnicode maps and permissible embedding flags, and deterministic trailer IDs. Historical certificate formats remain readable without being rewritten to claim PDF/A.

Section C, `Final Suno Generation`, labels each value by what it actually represents: final-generation date, Suno ID/final-generation ID, project URL, project/version ID when present, download/export date, metadata detection and origin, model, plan at generation, and the system-verified release/export SHA-256 comparison. A date is never labeled as if it were the generation object. The plan remains a user-confirmed fact unless a future implementation can identify a technically verified evidence source.

The source section preserves the code-audio chain as `Source code evidence` → `Code-generated audio evidence` → `Post-processing` and the selected processing operations. This relationship is factual provenance only; the presence of code or generated audio does not establish authorship. Likewise, `Release identical to Suno final export: YES` is emitted only from equal verified SHA-256 values, never from a matching filename, size, or date.

Terms summary and evidence-register detail refer to the same Evidence ID and file. The summary cannot say that Terms evidence exists while the register omits its title, provider/source, retrieval date, optional stored context, original filename, portable path, import timestamp, provenance, and full SHA-256. A verified local Terms file cannot coexist with `Terms evidence not available: YES`: the native API rejects that update, workflow consistency blocks an imported contradiction, and both certificate renderers reject it instead of producing conflicting output. Subscription coverage states only whether documented intervals jointly cover production and whether one contains final generation; it never confirms commercial rights or license validity.

The manifest, Markdown, and PDF share the sorted relative references of any earlier `.archive/revisions/<revision-id>/` entries that contain managed `revision.json` metadata. Manifest relationships use an explicit `derivedFromEvidenceId` when present. Without one, they link only a single unambiguous source and target in directly adjacent source-code/audio or artwork stages; ambiguous or skipped stages create no guessed ID-level lineage. Workspace-global copies retain `global_copy` provenance and `sourceGlobalEvidenceId` on the evidence record. A separate global-to-track relationship names that source record, the materialized local evidence ID, and the target track ID instead of misrepresenting it as an evidence-to-evidence derivation. These technical relationships do not invent a creative or legal conclusion.

The certificate uses these exact semantic boundaries:

| Representation | Meaning |
| --- | --- |
| `PASS` | Configured documentation requirements for this step were satisfied. |
| `DOCUMENTATION COMPLETE` | Configured documentation requirements for the finalized snapshot were completed. |
| `NO` | The user explicitly confirmed that the fact does not apply or did not occur. |
| `N/A` | The fact is logically inapplicable to this track and a reason is retained. |
| `NOT DOCUMENTED` | Sufficient documented information is absent. |

These labels do not confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory or AI-law compliance, or governmental certification. Facts are labeled as `User-confirmed fact`, `Evidence-derived metadata`, or `System verification`; absent facts remain `NOT DOCUMENTED` instead of being promoted to a technical verification.

## External timestamp addenda

Technical finalization and external timestamp attachment are two separate phases. The base certificate records its own application finalization time but does not call that time an independent timestamp. It records `No external timestamp evidence recorded at technical finalization`; this is expected because the external service can stamp an anchor only after phase one exists. A configured automatic action runs only after that immutable snapshot is committed. An external timestamp remains optional and its success is not a general finalization requirement.

Automatic RFC-3161 attachment fixes the referenced artifact to the exact finalized `06_CERTIFICATE/EVIDENCE_MANIFEST.json`; neither the UI nor provider chooses another hash. SunoDM resolves its expected SHA-256 from `CERTIFICATE_SHA256.txt`, rereads the manifest, and submits a SHA-256 TimeStampReq containing a fresh nonce and optional configured policy OID. It checks the same manifest anchor before and after the provider call. `VERIFIED` requires TimeStampResp success and TSTInfo version 1, exact SHA-256 imprint and nonce, a returned policy OID and requested-policy match when configured, CMS signed attributes and signature, a critical sole `id-kp-timeStamping` EKU, signer/chain validity at `genTime`, and a chain to explicitly configured TSA trust-anchor certificates. The addendum retains request/response nonce, policy values, verifier identity, signature/chain results, and trust-anchor SHA-256 fingerprints. SunoDM does not infer legal or eIDAS qualification from that technical result.

Manual/legacy attachment remains available for externally obtained evidence. It can reference one exposed stable anchor or a custom `Other` path that is still an exact verified phase-one `SHA256SUMS.txt` entry. It stores claimed and actual SHA-256 plus `Referenced hash match: YES` or `NO`, but it does not automatically establish provider identity, CMS signature, TSA EKU, or chain trust. An initial OpenTimestamps proof is locally bound to the requested digest and remains `ATTACHED` until separately verified or upgraded; it is not RFC-3161 verification.

Sidecar format v1 follows a durable stage → database registration → live publication sequence. The core set contains immutable `TIMESTAMP_RECORD.json`, managed timestamp evidence, Markdown addendum, PDF/A-2b addendum, and `TIMESTAMP_RECORD_SHA256.txt`; an adapter may additionally retain an exact raw provider response when the evidence is a derived wrapper. The staged file set and parent are synchronized before its certificate-bound SQLite row is created, using directory `fsync` where supported. Only a registered record is published. Recovery publishes matching registered stages, removes abandoned unregistered stages, and rejects unexpected unregistered live directories.

The immutable record pins its Certificate ID and finalization-snapshot ID, selected anchor, evidence and optional raw-response hashes, RFC request/response verification facts when applicable, publication-time integrity, and exact Markdown/PDF hashes. It excludes current `integrityVerified` and `integrityIssues`. The hash list protects every managed sidecar artifact except itself. The phase-one manifest, main hash list, Markdown certificate, both PDF certificates, and certificate hash list are never regenerated, so the stamped anchor remains byte-identical and no self-reference cycle exists.

On every load, the application resolves each registered current or archived record and checks the exact regular-file set, byte-identical canonical JSON registered by SQLite, every sidecar hash, pinned response/Markdown/PDF hashes, referenced phase-one artifact and stored match result, and archive-to-Certificate-ID binding. It hashes the published addendum bytes instead of re-rendering them. For an automatic RFC-3161 record, the UI reconstructs `VERIFIED` only if that intact sidecar still contains the complete positive provider predicate, is bound specifically to an equal claimed/actual Evidence Manifest anchor, uses the exact current verifier identifier, and retains well-formed pinned-root fingerprints. It does not rerun a network request or promote manual, legacy, OTS, incomplete, or stale summary state. Sidecar damage changes only that record's current integrity/status; the independent base certificate remains unchanged.

Each timestamp record is bound to the certificate ID of the revision on which it was attached. Revision creation archives the complete old `06_CERTIFICATE/` directory, including its timestamp addenda, and does not copy those records to the new certificate. The archive is accepted only when `revision.json.previous_certificate.certificateId` equals the sidecar Certificate ID. A new revision therefore has its own snapshot, hashes, and optional timestamp evidence.

After authoritative native commit, the UI may present a reusable certificate summary from the returned finalized `TrackDetail`. The summary does not create or reinterpret certificate data. It remains available in Finalize and the Certificate section only while the snapshot has a valid certificate ID, and links to the complete in-app certificate presentation.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-TRK-001` | Creating a track produces the required directory structure without fake evidence. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-TRK-002` | Conditional facts and evidence are required only when their controlling answers apply. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-TRK-003` | Interactions inside the new-track dialog retain the entered title, production-start date, and commercial-use choice; only a direct backdrop action or an explicit close/cancel control dismisses the dialog. | [ATP-0002](../atp/active/ATP-0002-track-creation.md) |
| `REQ-DOC-001` | Versioned templates generate all required factual documents deterministically. | [ATP-0004](../atp/active/ATP-0004-document-generation.md) |
| `REQ-EVD-001` | Evidence import preserves the source, records metadata, provenance, and a hash, and never silently overwrites. | [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) |
| `REQ-EVD-002` | Structured Suno WAV metadata is parsed within explicit bounds, retains its evidence/hash origin, derives only permitted dates, and never mutates a finalized snapshot. | [ATP-0015](../atp/active/ATP-0015-technical-evidence-certificate.md) |
| `REQ-ART-001` | Artwork stages follow the declared process and preserve the AI original. | [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) |
| `REQ-ART-002` | Applicable AI artwork receives a reproducible local visible disclosure whose source ID, generator version, exact text, and bytes remain traceable according to project policy. | [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md) |
| `REQ-HSH-001` | The correct included set is hashed and immediately verified; exclusions never enter the list. | [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md) |
| `REQ-CER-001` | A successful finalization writes a factual certificate, relative-path manifest, and certificate hashes. | [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) |
| `REQ-CER-002` | A post-finalization sidecar-v1 addendum is fully synchronized in staging, database-registered, then published. Automatic RFC-3161 uses the exact finalized Evidence Manifest and reports `VERIFIED` only for the complete imprint/nonce/policy/CMS/EKU/genTime/pinned-chain predicate; manual/legacy/OTS evidence stays distinct. Exact immutable sidecar bytes and certificate/revision binding are reverified without changing phase one, auto-adopting an orphan, or transferring evidence to another revision. | [ATP-0016](../atp/active/ATP-0016-evidence-certificate-workflow-5.md) |
| `REQ-AUD-001` | A current editable authoritative release source has a real bundled-Chromaprint fingerprint record and portable hash-covered summary; optional ACRCloud results are explicit, bounded, sanitized, and non-blocking. | [ATP-0017](../atp/active/ATP-0017-pre-release-audio-screening.md) |

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
- Audio-screening results are comparison records, not content or rights determinations; an unavailable bundled engine or provider remains non-positive.
- A user can intentionally remove portable files outside the application; the next scan or verification reports the resulting missing item or mismatch.

## Related documents

- [Product architecture](product-architecture.md)
- [Persistence and recovery](persistence.md)
- [Workflow model](workflow-model.md)
- [Pre-release audio screening](pre-release-audio-screening.md)
- [Finalizing a track](../usr/finalizing-a-track.md)
- [Legacy track import](../dev/legacy-track-import.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-20 | Added deterministic bounded ACRCloud multi-sample state, plan/coverage results, and manifest/certificate reporting; advanced the template to 1.11, manifest to 8, and certificate/PDF format to 6.1. | Project team |
| 2026-08-20 | Advanced template to 1.10, manifest schema to 7, and certificate/PDF format to 6.0; defined PDF/A-2b/full DejaVu embedding and the fixed-manifest cryptographically verified RFC-3161 sidecar path alongside distinct legacy/manual/OTS paths. | Project team |
| 2026-08-19 | Finalization now creates separate German and English certificate PDFs and advances the certificate format to 5.2. | Project team |
| 2026-08-18 | Added portable local/optional-external audio-screening state and artifacts; advanced template to 1.9, manifest to 6, and certificate/PDF to 5.1. | Project team |
| 2026-08-17 | Defined sidecar format v1, database-before-live publication recovery, immutable addendum-byte pinning, current/archived load verification, and the Terms availability invariant. | Project team |
| 2026-08-17 | Defined the workflow-1.7 split for instrumental/vocal/Suno-field content, complete Final Suno Generation and Terms facts, separate Audio/Artwork AI assessments, exact answer/status semantics, and post-finalization timestamp addenda; advanced template to 1.8, manifest to 5, and certificate/PDF to 5.0. | Project team |
| 2026-08-17 | Added workflow-1.6 download/last-editing derivation, joint subscription coverage, template 1.7, manifest schema 4, and certificate format 4.1. | Project team |
| 2026-08-17 | Added bounded Suno WAV metadata extraction, evidence-derived date/origin rules, generic byte identity, immutable revision analysis, template 1.6, manifest schema 3, and certificate format 4.0. | Project team |
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
