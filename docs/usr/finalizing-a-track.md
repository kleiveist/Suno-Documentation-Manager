<!-- AUTO-GENERATED:backlink START -->
[← Back](usr.md)
<!-- AUTO-GENERATED:backlink END -->
# Finalizing a track

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-20 |
| Audience | Users finalizing a track documentation set |
| Related ATP | [ATP-0008: Finalization gate](../atp/active/ATP-0008-finalization-gate.md); [ATP-0016: Evidence and certificate workflow 5.0](../atp/active/ATP-0016-evidence-certificate-workflow-5.md); [ATP-0017: Pre-release audio screening](../atp/active/ATP-0017-pre-release-audio-screening.md) |

## Purpose

This guide explains how to resolve finalization blockers, generate and verify integrity records, create the Track Documentation Completion Certificate, and safely handle a later change.

## Scope

### Included

- readiness review;
- document and artwork freshness;
- SHA-256 generation and verification;
- certificate and evidence-manifest creation;
- independent command-line verification; and
- invalidation and revision behavior.

### Excluded

- legal or copyright review;
- digital identity, government certification, or qualified signatures;
- modification of an archived revision; and
- remote publication or backup.

## Understand finalization

Finalization freezes the current documentation and integrity result as a versioned snapshot. It does not certify authorship or legal compliance. The application enables `FINALIZE DOCUMENTATION` only after native validation finds no blocker.

The gate requires:

- every applicable mandatory step to be `PASS` or justified `N/A`;
- no open blocking deviation;
- every required evidence role to contain a real readable file;
- Suno Instrumental Mode, scalar Content Classification, explicit Vocal Intent, and final-audio vocal presence to be answered independently; `EMPTY` needs no text/source, while every other classification needs exact text and source and `OTHER` additionally needs a label;
- the actual release and Suno-export filenames to match the title or have an explicit intentional-deviation confirmation;
- evidence-derived dates and their recorded evidence/hash origins to match the current Suno export;
- the concrete final-generation date of a commercial track to fall inside selected subscription coverage; and
- commercial tracks to include archived Suno Terms evidence with document title, provider/source, and retrieval date, without simultaneously claiming that Terms evidence is unavailable;
- the Audio AI assessment to be complete when generative AI is used, including a disclosure status other than `NOT DOCUMENTED` for commercial intent;
- a current local Chromaprint fingerprint record to be bound to the authoritative release evidence;
- all generated documents to match current facts, evidence metadata, and template versions;
- SHA-256 generation to cover the complete required set;
- native verification to pass for every listed file; and
- workflow and application versions to be available for the certificate.

Register an archived Suno Terms PDF once under `Einstellungen` → `Globale Datei-Führung`. Enter its title, provider/source, and retrieval date; add a source URL, effective date, applicable production period, or factual note only when known. The native picker accepts a signature-checked PDF and places a portable global-evidence copy with the same Evidence ID and metadata into every editable project. Once verified local Terms evidence is attached, SunoDM rejects an attempt to set `Terms evidence not available` to `YES`. A contradictory imported legacy value blocks workflow consistency and certificate generation instead of appearing beside `Terms evidence exists: YES`. If a finalized project needs newer or corrected Terms, create a revision first; finalized snapshots are never rewritten.

An external timestamp is not a prerequisite of this gate. After the gate has produced the stable Evidence Manifest anchor, an enabled automatic provider attempt runs inside the finalization transaction before the certificate is rendered. Provider, verification, or qualification-lookup failure is captured in the final certificate but does not turn the documentation gate into a failure. `FAIL`, `BLOCKED`, `NOT VERIFIED`, or an N/A item without a reason in the mandatory workflow still prevents finalization.

The optional ACRCloud comparison is also not a gate. It is available only from Step 07 after `Einstellungen` → `Externe Dienste` configuration and an explicit user action. A disabled/unconfigured provider, an unavailable provider, an unsupported format, or an authentication failure is a visible non-positive technical state, not a finalization blocker and not a match result.

## Resolve missing items

Open the track overview and follow each missing-item action. Common blockers include:

- a missing Suno project URL;
- no final release WAV or other configured final release role;
- a missing AI artwork original after AI artwork was declared;
- no selected subscription evidence for the production period;
- an archived Terms file whose title, provider/source, or retrieval date is missing for a commercial track;
- an unanswered conditional ownership or license question;
- a missing scalar Content Classification or missing explicit Vocal Intent;
- commercial generative-AI use whose audio-disclosure status remains `NOT DOCUMENTED`;
- a user-confirmed final-generation date that conflicts with valid embedded Suno metadata;
- an evidence-derived date whose source evidence was replaced or removed;
- a stale generated document after an input changed;
- an unprocessed visible AI disclosure required by project policy; or
- an integrity mismatch.

Do not enter a guessed historical value or attach an unrelated file merely to clear a blocker. A truthful `NOT VERIFIED` state is preferable to fabricated documentation.

The Suno final export, final release audio, and final artwork are each singular. Use the adjacent replacement action when one changes so generated documents and the certificate cannot refer to ambiguous authoritative assets.

## Review lyrics and Final Suno Generation

Do not use text in Suno's Generation Text Field as proof of the final audio. Review four independent facts: Suno Instrumental Mode, the scalar Content Classification, Vocal Intent, and whether the final audio contains vocals. Choose `MIXED` for Vocal Lyrics plus structure instructions. `EMPTY` is the only N/A content branch; otherwise confirm the `human`/`AI`/`mixed` source and exact text, plus a label for `OTHER`. `UNSPECIFIED` is a legitimate explicit Vocal Intent choice, distinct from an unanswered field. Intent/output differences do not block finalization.

Review Section C as a chain of distinct facts: final-generation date, Suno ID/final-generation ID, project URL, project/version ID when present, download/export date, metadata origin/detection, model, plan at generation, and `Release identical to Suno final export`. A user-confirmed plan such as `Premier` is not evidence-derived unless explicitly labeled otherwise. The later coverage comparison only establishes whether documented date intervals contain the production/final-generation dates; it does not confirm commercial rights.

## Review automatic Suno metadata

When you import a Suno final-export WAV, the application checks its bounded structured metadata. It recognizes only the complete case-insensitive marker segments `made with suno studio` and `made with suno`. Derivation requires one marker, one RFC 3339 `created` value, and one UUID `id` in the same unambiguous record; missing, malformed, duplicated, mixed, incidental, or unsafe values do not become automatic facts. This check documents metadata found in the local WAV and does not authenticate Suno or the provider.

The calendar part of `created` is authoritative for the final-generation date in Step 03, the production-end date in Step 01, and the optional download/export date in Step 03. As long as a valid metadata date exists, these values are filled automatically and shown read-only. Step 07 asks whether the WAV was edited again on the desktop PC. Choose `No` to derive and lock the last-editing date from the WAV; choose `Yes` to enter the actual date and confirmed editing work yourself. You can enter manual fallback dates when no valid metadata date is available. The application never substitutes an import or filesystem timestamp.

Check the origin shown next to an automatic value. Replacing the Suno export updates every applicable automatic date; removing it clears the evidence-derived values and restores manual fallback inputs. An ordinary WAV without the structured marker remains usable evidence and does not create generation facts.

A finalized certificate is never rewritten when marker support changes. Start an explicit revision to analyze the carried, hash-matching WAV with the current detector. The former certificate, manifest, and any external-timestamp sidecars remain archived with their original bytes; the new manifest requires its own timestamp attachment.

For commercial tracks, attach each globally registered receipt whose interval overlaps production or covers final generation. A single receipt need not cover everything: adjacent intervals are combined, but the full production period must remain gap-free and final generation must fall inside at least one attached interval. The list marks partial production overlap as `TEILWEISE` instead of rejecting that receipt.

The overview also reports hash-based byte identity. `Release identical to Suno export` means the verified release audio and Suno final export have exactly the same SHA-256; matching names or dates alone do not produce that result. Identity is a technical observation, not a legal conclusion and not a requirement that the files must be identical after documented editing.

## Review pre-release audio screening

When the authoritative release evidence is imported or replaced in an editable track, SunoDM runs its bundled, version-pinned Chromaprint engine against the managed file. Review the local status in Step 07 and confirm that `LOCAL_FINGERPRINT.json` records the current source Evidence ID, track-relative source path, SHA-256, size, measured duration, engine/version, algorithm, and generation time. The full fingerprint is retained only in that dedicated local JSON record; it is deliberately not shown in the track overview, manifest, certificate, or PDF.

The generated `03_DOCUMENTATION/AUDIO_SCREENING/AUDIO_SCREENING.md` provides the portable concise summary. The local JSON, its detached SHA-256 file, and any explicitly created external result/response artifacts are included in the normal phase-one SHA-256 list. If the release evidence changes, prior screening state is stale and a local run must bind the new bytes before finalization. An unavailable bundled engine or unsupported input remains a controlled non-positive state; SunoDM does not fall back to a system-installed tool or invent a fingerprint from the file hash.

ACRCloud is optional. Under `Einstellungen` → `Externe Dienste`, enable the provider, enter the host and write-only credentials, save, and use the provider test to check reachability. In Step 07, choose the explicit external screening action only when you intend to send a bounded release-audio sample to that configured provider. The app does not upload the Chromaprint fingerprint and does not send credentials or request signatures into portable documentation. When a response is received, the structured `ACRCLOUD_SCREENING.json` and a credential-safe `ACRCLOUD_RESPONSE.json` are archived under `03_DOCUMENTATION/AUDIO_SCREENING/` and integrity-protected. A provider match or no-match is a technical response fact only; it does not establish authorship, rights, permission, infringement, legality, or release clearance.

## Review AI transparency

Review the Audio assessment independently from Artwork. If generative AI was used, confirm the named AI system and all six tri-state indicators: AI-assisted elements, AI-generated elements, intentional real-person voice imitation, intentional identity representation, authentic-recording representation of a real event, and authentic AI-recording presentation of a real location/institution/event. `NO` is a deliberate negative answer. `NOT DOCUMENTED` records missing information and must never be read as `NO`.

Review the audio-disclosure answer separately. `YES` requires the recorded locations and exact text. `NO` is a conscious answer and can include a factual reason; SunoDM does not decide whether that reason is legally sufficient. For a commercially intended track using generative AI, `NOT DOCUMENTED` remains incomplete and blocks finalization. The certificate may state whether a potential indicator was recorded, but it never states `No deepfake`, `AI Act compliant`, or `Disclosure legally unnecessary`.

If the artwork is AI-generated or AI-assisted, review the original, any intermediate stages, the configured project transparency policy, disclosure decision, text, and final output. Every AI artwork requires an explicit `YES` or `NO`, even when all three content checks are negative or the configured policy is `none`. `NO` is retained as a conscious non-application. For `YES`, generate the visible local disclosure before choosing the final release artwork.

Confirm that:

- the AI original still exists unchanged;
- the output with disclosure is a separate file;
- the disclosure is visible and uses the configured text and placement;
- the disclosed output has `generated_disclosure` provenance, identifies the verified AI-original evidence as its source, records generator version `local-disclosure-v1`, and retains the exact configured text;
- the imported final artwork is exactly that locally generated disclosed output (the native gate compares its SHA-256); a manually imported edited image or unrelated unmarked final image is blocked;
- `AI_USAGE.md` and `artwork_process.md` identify the service, base image, human modifications, policy, result, text, and final output; and
- positive real-person, real-event, trademark, or logo answers have the configured factual note or evidence.

If verified human-edited artwork and the single verified final artwork share their SHA-256, the technical output records `BYTE-IDENTICAL / SHA-256 MATCH`. A different hash is information, not a blocker. New human-edited imports use `_HUMAN_EDITED`; existing `_EDITED` evidence and all finalized snapshots retain their paths. Treat every Artwork import timestamp only as the time of import into SunoDM, never as evidence of the actual creation or editing sequence.

The Artwork content check records your answers. It does not decide legality, and three negative Artwork answers do not hide the Audio assessment or complete the Artwork disclosure decision.

## Generate current documents

Use the document-generation action after completing the relevant steps. Template `1.11` includes current evidence metadata and the current audio-screening state in the deterministic document snapshot; the manifest and certificate retain the automatic fact origins, separated lyrics/AI facts, complete Terms context, final-Suno verification summary, and a concise multi-sample screening summary without raw fingerprints or provider secrets. Review the generated Markdown and text files for factual accuracy. The documents should state confirmed facts, exact `NO`/`NOT DOCUMENTED` answers, and applicable N/A reasons, not legal guarantees.

While the native service prepares, renders, and atomically writes the managed documents, the progress view shows the current phase, elapsed time, current relative path, and completed document count. These are live operation values rather than a simulated upload. The animated scene is only a visual companion; the final native result remains authoritative. Generated headings and guided values remain English even when the interface and progress view are German.

If an unmanaged document already exists at a managed destination, the application first shows the existing state. It writes managed content only after explicit confirmation and a backup below `.archive/`. Resolve any collision before continuing.

After changing a source answer or evidence selection, regenerate affected documents so the application no longer reports them as stale.

## Generate and verify SHA-256

Use the integrity step to generate `03_DOCUMENTATION/SHA256SUMS.txt`. The application hashes release files, Suno evidence, documentation, licenses, and artwork. It excludes `.archive/`, `.summary/`, the hash list itself, `06_CERTIFICATE/`, and the exact root-level `SunoDM_DOCUMENTATION_CERTIFICATE.pdf` and `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf` paths. The PDFs are created later and protected by `CERTIFICATE_SHA256.txt`. Workspace `.suno-doc/` data sits outside the track root; a nested directory with that name inside a track is normal protected content.

Generation is not enough. The native service immediately rereads each listed file. Continue only when the displayed generated and verified counts match and the result is `PASS`.

The progress view reports the current relative file, processed file count, bytes read, elapsed time, and current phase. During generation the meter first follows the real bytes used to calculate the new hashes and then advances through the immediate second read used for verification. A separate verification action reports its own real reread progress. Large files can therefore remain on one phase for some time without the application being frozen. Keep a removable drive connected until the operation has completed.

The orbit, scan, and data-stream animations provide visual activity while the native work continues. If the operating system requests reduced motion, the application disables those repeating animations while keeping all numeric progress and status information visible.

For an independent check on a system with `sha256sum`, run this command from the track root:

```sh
sha256sum -c 03_DOCUMENTATION/SHA256SUMS.txt
```

This external check is optional product-independent verification. The normal application workflow uses native Rust and does not require the shell command.

## Finalize

Select `FINALIZE DOCUMENTATION`. The native layer reevaluates the complete gate; it does not trust a cached UI percentage. A successful transaction creates:

```text
SunoDM_DOCUMENTATION_CERTIFICATE.pdf
SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf

06_CERTIFICATE/
├── DOCUMENTATION_CERTIFICATE.md
├── EVIDENCE_MANIFEST.json
└── CERTIFICATE_SHA256.txt
```

`SunoDM_DOCUMENTATION_CERTIFICATE.pdf` (English) and `SunoDM_DOCUMENTATION_CERTIFICATE_DE.pdf` (German) format 6.2 are A4, A–L PDF/A-2b technical representations of the same finalized snapshot. Both PDFs are created automatically; the Step 10 language setting no longer controls whether the second PDF is created. They separate the documented title from both original filenames; give a complete Final Suno Generation section with distinct dates, IDs, URL, model, plan, origins, and release/export hash comparison; render every source branch with N/A where appropriate; render Section F as `Suno Generation Text Field` with separate text-field availability, usage, scalar Content Classification, independent Vocal Intent, exact field content, and final-audio vocal outcome; record only selected human work; separate Audio and Artwork AI answers; and list technical subscription coverage, the same Terms Evidence IDs used by the evidence register, provenance, lineage, full SHA-256 values, and K.2 pre-release audio-screening source/artifact facts. K.2 now records the configured intensity/mode, target and executed coverage, and every sample's bounded offset/result without printing a raw fingerprint, raw provider response, signature, or secret. The Certificate ID and `Seite X / Y` appear on every page. They are technical documentation, not legal or governmental certification.

Each current PDF embeds the complete DejaVu 2.37 `DejaVuSans`, `DejaVuSans-Bold`, `DejaVuSansMono`, and `DejaVuSansMono-Bold` font programs under the DejaVu Fonts License; it does not depend on fonts installed on the review system. XMP identifies PDF/A-2b and the document includes the CMYK FOGRA39 output intent. Manifest schema 9 records the archive profile, `full` embedding mode, font names and SHA-256 values, font version/license, and output intent. See the [track documentation model](../def/track-documentation-model.md#certificate-artifacts) for the exact bundled font hashes.

Section I now records the actual result present at the single certificate generation. SunoDM serializes `EVIDENCE_MANIFEST.json`, computes its SHA-256 anchor, performs at most one configured automatic request, and then renders the Markdown plus both final PDFs exactly once. Section I separates provider configuration, the concrete timestamp result, protocol/hash/signature/chain checks, and provider trust/qualification. A later explicit retry creates another immutable addendum and never rewrites the base PDF.

During finalization, the progress view distinguishes native gate validation, snapshot collection, transaction protection, certificate/manifest generation, certificate verification, the complete SHA-256 reread, and the final database commit. File names, byte counts, file counts, and elapsed time remain visible while the filesystem work runs outside the Tauri main thread.

After the verified native result is committed, a certificate summary opens automatically. It shows the certificate ID, track, artist, finalization time, workflow, verified integrity count, evidence count, blocking-deviation count, and final result. Close it to continue, or open the complete certificate view. For a still-valid finalized track, the same summary remains available through `Show certificate` in Finalize and `Open certificate summary` in the Certificate section.

Review the certificate ID, track, artist, workflow and application versions, application finalization time, mandatory-step result, N/A reasons, evidence count, selected hashes, blocking-deviation result, earlier revision references, and final status. The application finalization time is not represented as an independent trusted timestamp. Expected success status is `DOCUMENTATION COMPLETE`.

Review manifest schema 9 and confirm that paths are track-root-relative. `documented_facts` contains the full user-facing track-fact snapshot, including distinct instrumental/vocal/Suno-field values and Audio/Artwork AI assessments. Each evidence item includes the original import filename, path, size, full hash, import timestamp, provenance, lineage, technical audio facts, structured embedded metadata when present, and complete stored Terms context for the Terms Evidence ID. The dedicated `evidence_derived_metadata` section retains the selected Suno timestamp and ID, while `system_verification` records detection, all date origins, joint production/final-generation subscription coverage, every byte-identical evidence pair (including the explicit human-edited/final-artwork result), the release/export identity result, unambiguous role relationships, explicit global-evidence-to-track relationships, and consistency issues. `certificate.pdf_archive` records the PDF/A/font profile. The sanitized `audio_screening` section records local/external status, source linkage, engine/provider identifiers, intensity/mode/coverage, per-sample offsets/results, artifact paths/hashes, and concise provider matches. It excludes the raw local fingerprint, raw provider response, request signature, and credentials. The schema-9 manifest records the prepared anchor and the ordering `after_manifest_anchor_before_single_certificate_render`; the actual attempt status is recorded in the final Markdown/PDF because it is known only after the manifest bytes are fixed. Successful provider bytes and later retries live in immutable sidecars and never modify the manifest anchor. The only externally obtained phase-one field is a response from the separately explicit ACRCloud screening request; neither a filename nor a network response is used to invent a fact outside its stated origin.

A project/version ID and a user-confirmed final-generation ID are shown only when supplied. The evidence-derived Suno ID remains separately labeled so different identifiers are not conflated. Compatibility time and lyrics fields from older records remain readable but do not satisfy new semantic answers or become finalization facts automatically.

The certificate defines its completion labels narrowly:

- `PASS`: Configured documentation requirements for this step were satisfied.
- `DOCUMENTATION COMPLETE`: Configured documentation requirements for the finalized snapshot were completed.
- `NO`: the user explicitly confirmed a negative fact.
- `N/A`: the fact is logically inapplicable and its reason is retained.
- `NOT DOCUMENTED`: sufficient documented information is absent.

None of these labels means rights cleared, copyright confirmed, legally complete, AI Act compliant, court-proof, or government certified.

The certificate includes this mandatory meaning:

> This certificate confirms the recorded inputs, finalized snapshot, registered evidence, recorded provenance, SHA-256 values, and configured workflow checks. It does not confirm authorship, rights ownership, non-infringement, legality, license validity, judicial evidentiary weight, statutory compliance, or governmental certification.

## Verify certificate artifacts

From the track root, a system with `sha256sum` can independently check the certificate integrity list:

```sh
sha256sum -c 06_CERTIFICATE/CERTIFICATE_SHA256.txt
```

The certificate list covers the main SHA-256 list, evidence manifest, certificate Markdown, and both root-level technical PDFs. It does not hash itself, and neither PDF contains a self-hash. Changing one PDF byte therefore makes native certificate verification fail.

## Attach optional external timestamp evidence

Automatic timestamping is part of the single-render finalization sequence, but it is not part of the mandatory documentation gate. When `Automatically during finalization` is enabled, SunoDM fixes the anchor to the exact SHA-256 of the already serialized `06_CERTIFICATE/EVIDENCE_MANIFEST.json`; you do not choose or enter that digest. It performs exactly one provider request before rendering the Markdown and both PDFs. A timeout, provider error, malformed response, missing trust anchor, failed concrete verification, or unavailable qualification source becomes factual non-positive data in those final artifacts. It does not block `DOCUMENTATION COMPLETE`. If a response succeeds, the exact same response bytes are archived and verified as an immutable sidecar before the final database commit; no second provider request is made.

For an RFC-3161 provider, `VERIFIED` means all of these technical checks passed:

- the response status is successful and TSTInfo is version 1;
- the SHA-256 message imprint equals the exact finalized manifest digest;
- the response nonce exactly equals the fresh request nonce;
- the provider returned a policy OID and it equals the requested custom policy when one was configured;
- the CMS signed attributes and signature verify with a supported declared algorithm;
- the TSA certificate has a critical Extended Key Usage containing only timestamping, is valid at `genTime`, and builds to the explicitly configured TSA CA trust-anchor file; and
- the provider response passes its local structure and binding checks; the same response bytes are then sidecar-verified before the final database commit.

The final certificate and addendum show the request/response nonce and policy, cryptographic verifier, signature/chain results, and SHA-256 fingerprints of the configured trust anchors. This technical verification result is independent from provider qualification.

Provider trust is evaluated from the cryptographically verified signer certificate and optional policy/service identifiers, never from the freely configured provider name, endpoint, URL, domain, or marketing description. Section I therefore permits `Timestamp: VERIFIED` together with `eIDAS qualification: NOT VERIFIED`, and also keeps a known provider qualification separate from a failed concrete timestamp.

A higher eIDAS result is shown only when the signer/service identity matches a service in an already cryptographically validated official Trusted List snapshot. The audit records the list source, territory, version, sequence, issue/next-update times, SHA-256, validation time/status, recognized Trust Service Provider and service, service type/identifier, and separate service-status URI/period evidence for timestamp time and check time. The official qualified electronic time-stamp service type and the matching granted-status identifier/period must support every positively reported time axis before it is rendered; the `eIDAS QUALIFIED TRUST SERVICE - VERIFIED` badge remains specific to the timestamp-time result.

A website lookup or successful HTTPS request is not qualification evidence. When no validated Trusted List snapshot is available, the current adapters conservatively report `NOT CHECKED`, `NOT DOCUMENTED`, `NOT VERIFIED`, or `CHECK FAILED`; they do not say that the provider is unsafe or not qualified. Trusted-List unavailability and missing historical status never change an otherwise successful technical timestamp and never block finalization. Credentials remain only in private local configuration and are not copied into any certificate, manifest, sidecar, trust record, revision, or log.

You can also obtain evidence outside SunoDM and use the manual post-finalization attachment. The Certificate view exposes stable anchors for the Evidence Manifest, main SHA-256 list, Markdown certificate, English PDF certificate, and certificate hash set. `Other` requires an explicit phase-one relative path whose current SHA-256 still exactly matches its entry in verified `03_DOCUMENTATION/SHA256SUMS.txt`; an arbitrary contained, excluded, added, or changed file is rejected. Enter:

- provider/issuer;
- type: `Qualified electronic timestamp – user declared`, `Electronic timestamp`, `External integrity timestamp`, `Other`, or `Not documented`;
- timestamp value when present;
- the selected referenced artifact and claimed SHA-256;
- the local timestamp evidence file; and
- optional external reference ID, provider verification URL, and factual note.

A qualified type is your declared classification; SunoDM does not report it as technically verified qualification. The native operation verifies the current certificate/integrity set, recalculates the selected local artifact, and stores both your claimed hash and the actual local hash. A mismatch is retained as `Referenced hash match: NO`; it is not hidden and produces no positive integrity claim. Manual/legacy evidence has no automatic provider-identity, CMS-signature, TSA-EKU, or chain verification and is never promoted to RFC-3161 `VERIFIED`. An initial OpenTimestamps proof is `ATTACHED` until it is separately verified or upgraded; it is not represented as RFC-3161 verification.

Each successful attachment creates a separate certificate-ID-bound sidecar:

```text
06_CERTIFICATE/EXTERNAL_TIMESTAMPS/<timestamp-record-id>/
├── TIMESTAMP_RECORD.json
├── TIMESTAMP_EVIDENCE.<original-extension>
├── PROVIDER_RESPONSE.<extension>            # only when separately archived
├── EXTERNAL_TIMESTAMP_ADDENDUM.md
├── EXTERNAL_TIMESTAMP_ADDENDUM.pdf
└── TIMESTAMP_RECORD_SHA256.txt
```

This is sidecar format v2. Historical v1 and v0 records remain readable and are never rewritten in place. `PROVIDER_RESPONSE.<extension>` is absent when `TIMESTAMP_EVIDENCE` already is the untouched provider response, as for RFC-3161; it is present only when an adapter also needs to preserve a distinct raw response, as for an initial OpenTimestamps calendar result. SunoDM first creates and verifies the complete managed set in `.archive/timestamp-staging/` and synchronizes the stage plus its parent, then registers its certificate-bound record in SQLite, and only then publishes the staged directory to the live path above. If live publication must be rolled back, removal from the live parent is synchronized before its database row is deleted; otherwise the row remains available for recovery. If the application stops after registration, reopening the workspace verifies and publishes the matching pending stage. A stage with no registered record is an abandoned operation and is removed. An unexpected live sidecar with no registered row causes a controlled error and is never auto-adopted.

The PDF addendum is also PDF/A-2b with the fully embedded DejaVu 2.37 font set. Both addenda show provider, type, timestamp value, referenced artifact/path, claimed and actual SHA-256 values, match result, evidence filename/hash, import time, optional reference/URL/note, provenance, and the bound Certificate ID. Automatic records additionally show their finalization-snapshot binding and provider verification facts. Immutable `TIMESTAMP_RECORD.json` records `sidecarFormatVersion: 2`, `integrityVerifiedAtPublication`, pinned response/Markdown/PDF hashes, and the provider predicate when present. It does not store current `integrityVerified` or the issue list. Its canonical bytes are checked exactly; inserting a runtime integrity or trust claim is detected even if the sidecar hash list is recalculated. They include this boundary:

> The application records technical timestamp evidence separately from provider qualification. It does not infer legal effect; a regulatory qualification is reported only when independently verified.

The sidecar hash list protects the record, copied evidence, Markdown, and PDF, but not itself. The original manifest, main hash list, base Markdown/PDF certificate, and certificate hash list remain byte-identical, so attaching evidence cannot change the hash that was timestamped. This avoids a cyclic hash dependency. From the timestamp-record directory, an optional independent check is:

```sh
sha256sum -c TIMESTAMP_RECORD_SHA256.txt
```

When you reopen the track, SunoDM reverifies the sidecar against its registered record: the exact managed regular-file set, canonical immutable JSON, exact published record/evidence/optional-response/Markdown/PDF bytes, all pinned hashes, referenced artifact and stored match result, versioned hash list, and archived Certificate-ID binding when applicable. It verifies published bytes; it does not re-render an older addendum or repeat a network request. An automatic RFC-3161 summary remains `VERIFIED` only when the intact record is bound specifically to an equal claimed/actual Evidence Manifest hash and still contains every positive nonce, policy, signature, chain, verifier, and pinned-root result. Missing, stale, summary-only, manual, legacy, or OTS state is not promoted. If a sidecar file was changed or removed, that timestamp record reports integrity `NO`, while the unchanged base certificate remains separately valid.

No external timestamp is required for ordinary finalization. When the automatic action is disabled or produces no evidence, the one-time certificate records the concrete state as `NOT RECORDED` with its separate provider-configuration and qualification statuses. A later explicit retry may add current timestamp evidence but cannot rewrite that historical PDF; the optional recommendation is not legal advice.

The Certificate view keeps the immutable single-render finalization result and any current or later timestamp sidecars visibly separate. Its automatic-consistency summary is presentation-only: optional observations are `INFO`, an attached but unverified OpenTimestamps proof is a `WARNING`, and existing mandatory contradictions remain `BLOCKING`. The aggregate is `PASS`, `PASS WITH WARNINGS`, or `BLOCKED` respectively. These labels do not change the workflow gate, `DOCUMENTATION COMPLETE`, certificate validity, or previously published files. After an initial OpenTimestamps attachment, switching Settings to a ready RFC-3161 provider permits an additional RFC-3161 sidecar; the OTS record is preserved rather than replaced.

## Preserve the snapshot

Keep the complete track folder together. The folder is designed to remain reviewable without the SQLite index or application. A copy that omits evidence or generated documents is not the same finalized snapshot.

Do not edit a finalized file in place. If a release asset, evidence file, generated document, or other protected input changes outside the application, the next integrity check reports that the certificate no longer matches.

## Create a revision after a change

Every finalized track is shown as a read-only snapshot. Its workflow rail, tabs, Dashboard, Tracks, Workspace, and Settings remain navigable, and evidence previews plus non-mutating integrity verification remain available. The application does not parse historical WAV evidence merely to backfill a newly supported fact, and it does not repeatedly attempt to save a finalized form while navigating. Actions that would change fields, evidence metadata, generated documents, hashes, or step status stay disabled until a revision exists.

Use `Create new revision and edit` from the overview, any workflow step (including Integrity and Finalize), or the certificate view when you intend to document a new snapshot. This action is available for both valid and invalid finalized certificates. Do not invalidate a valid certificate merely to make the revision action appear.

When the application reports `Documentation changed after finalization`, review the mismatch before proceeding. Then create the revision explicitly.

The application archives `revision.json`, the prior `03_DOCUMENTATION/SHA256SUMS.txt`, the complete former `03_DOCUMENTATION/AUDIO_SCREENING/` directory, the complete former certificate directory including its `EXTERNAL_TIMESTAMPS/` sidecars, and both former root-level technical PDFs below `.archive/revisions/<revision-id>/`, then opens a new working revision. It can preserve this recovery record even when the live certificate was already damaged. Every timestamp record remains bound to the archived Certificate ID and is never copied or reassigned to the new revision. Audio-screening records likewise remain with their archived source snapshot; an ACRCloud result is never silently transferred to a new revision. Archived records remain listed and are reverified from their archived bytes whenever the track loads; `revision.json.previous_certificate.certificateId` must equal the sidecar Certificate ID. Modifying that binding or a sidecar byte makes the timestamp record's current integrity `NO` without changing the base certificate's validity. The mutable revision may analyze carried Suno WAV evidence and derive its permitted dates; the archived finalized revision is not changed. After the next successful finalization, the current manifest, Markdown certificate, and PDF list the relative paths of these managed earlier revision archives. Update the relevant facts or evidence, regenerate documents, apply artwork disclosure if required, regenerate and verify hashes, and pass the complete finalization gate again. Attach a new external timestamp only after that revision has its own final certificate and anchors.

Tracks created by an older application version or recovered from an imported folder may not yet contain `.archive/revisions/`. Revision creation safely creates this managed parent before moving any live certificate artifact; users do not need to create the folder manually.

If the application or machine stops during certificate publication, reopening the workspace uses the matching `.archive/finalization-in-progress.json` marker to identify only that application transaction. A published certificate directory, both root-level PDFs, and the correlated staging set beside a non-finalized record are then moved to `.archive/recovery/<transaction-id>/` with recovery metadata. A stale marker beside an already finalized record is removed. Historical certificate files without this marker are left untouched and are not assumed to be a failed application finalization.

A new workflow version does not modify an older certificate. The application shows the finalized and current workflow versions and requires explicit reevaluation.

## Verification

Use a non-sensitive test track and record actual results only in the acceptance protocol. Verify at least one blocked attempt and one eligible attempt:

1. Leave one mandatory evidence role empty and confirm finalization is blocked with the exact missing item.
2. Complete every mandatory item, regenerate documents, and generate and verify hashes.
3. Finalize and inspect the three files under `06_CERTIFICATE/` plus both root PDFs at the track root.
4. Independently verify both SHA-256 lists where the platform tool is available.
5. Attach disposable timestamp evidence once with the displayed manifest hash and once with a deliberately different claimed hash; confirm `Referenced hash match: YES` and `NO` remain distinct without changing the base certificate bytes.
6. Modify a protected disposable file and confirm certificate invalidation.
7. Create a revision and confirm that the previous certificate and its timestamp sidecars remain archived and are not attached to the new revision.
8. Import or replace a disposable release file, confirm that a current local fingerprint is generated, and verify that its portable screening records occur in `SHA256SUMS.txt` without exposing the raw fingerprint in the certificate artifacts.
9. With a fixture provider, verify explicit ACRCloud no-match, match, authentication-failure, and unavailable-provider results; confirm that none blocks finalization or appears as a legal conclusion.
10. Verify a technically valid RFC-3161 response without validated Trusted List evidence; confirm the PDF says technical `VERIFIED` and qualification `NOT VERIFIED` or `NOT CHECKED`, and that changing a custom provider label/URL cannot create an eIDAS badge.
11. With a cryptographically validated Trusted List fixture, verify signer-certificate matching, the qualified service type/status, timestamp-time versus current status, and the positive badge; then make the source unavailable and confirm that finalization and the concrete technical result still succeed.

Executed results are recorded in [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md), [ATP-0008](../atp/active/ATP-0008-finalization-gate.md), [ATP-0009](../atp/active/ATP-0009-certificate-generation.md), [ATP-0010](../atp/active/ATP-0010-certificate-invalidation-and-revision.md), current [ATP-0016](../atp/active/ATP-0016-evidence-certificate-workflow-5.md), [ATP-0017](../atp/active/ATP-0017-pre-release-audio-screening.md), and the historical [acceptance report](../dev/acceptance-report.md).

## Related documents

- [Getting started](getting-started.md)
- [Track documentation model](../def/track-documentation-model.md)
- [Workflow model](../def/workflow-model.md)
- [Persistence and recovery](../def/persistence.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-21 | Added single-render format 6.2 / manifest schema 9 timestamp reporting, provider-neutral signer identity, separate technical and eIDAS qualification states, historical/current Trusted List audit fields, and sidecar v2. | Project team |
| 2026-08-20 | Added configurable deterministic ACRCloud multi-sample intensity, request/coverage limits, per-sample response archives, and template 1.11 / manifest 8 / certificate 6.1 reporting. | Project team |
| 2026-08-20 | Documented workflow 1.9/template 1.10/manifest 7/certificate 6.0, PDF/A-2b with fully embedded DejaVu fonts, and the automatic fixed-manifest RFC-3161 verification path separately from manual/legacy/OTS attachments. | Project team |
| 2026-08-19 | Finalization now creates separate German and English technical certificate PDFs automatically; removed the Step 10 language switch. | Project team |
| 2026-08-18 | Added pre-release local Chromaprint and explicit optional ACRCloud screening guidance, including snapshot/revision behavior and formats 1.9/6/5.1. | Project team |
| 2026-08-17 | Documented sidecar-v1 staging, registration, publication and startup recovery; immutable pinned-byte verification for current and archived addenda; and rejection of contradictory Terms availability. | Project team |
| 2026-08-17 | Documented workflow 1.7 finalization semantics, template 1.8, manifest schema 5, certificate/PDF 5.0, separated lyrics and AI facts, complete Terms metadata, and stable certificate-bound external-timestamp addenda without cyclic hashing or legal qualification claims. | Project team |
| 2026-08-17 | Documented workflow 1.6 derived download/last-editing dates, Step-07 desktop editing, joint subscription coverage, and the 1.7/4/4.1 artifact versions. | Project team |
| 2026-08-17 | Documented workflow 1.5 authoritative read-only dates in Steps 01 and 03, with manual fallback only when no valid Suno metadata date exists. | Project team |
| 2026-08-17 | Documented Suno WAV metadata review, conditional date derivation, optional download date, consistency blockers, byte identity, revision-only analysis, and the 1.4/1.6/3/4.0 artifact versions. | Project team |
| 2026-08-15 | Added finalization progress and the automatic, reusable certificate-summary dialog. | Project team |
| 2026-08-16 | Documented workflow 1.3 gates and certificate 3.0 final-generation, terms, timestamp, origin-label, and disclaimer content. | Project team |
| 2026-08-15 | Documented live document, SHA-256, and verification progress, including the immediate second hash pass and reduced-motion behavior. | Project team |
| 2026-08-15 | Documented automatic revision-parent repair for older and imported tracks. | Project team |
| 2026-08-15 | Clarified read-only finalized navigation and the directly available revision action. | Project team |
| 2026-08-13 | Added portable disclosure-lineage review and marker-scoped finalization recovery. | Project team |
| 2026-08-13 | Documented revision archive contents and interrupted-operation recovery. | Project team |
| 2026-08-13 | Added the finalization, certificate-verification, and revision guide. | Project team |
