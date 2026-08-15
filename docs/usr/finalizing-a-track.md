<!-- AUTO-GENERATED:backlink START -->
[← Back](usr.md)
<!-- AUTO-GENERATED:backlink END -->
# Finalizing a track

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-15 |
| Audience | Users finalizing a track documentation set |
| Related ATP | [ATP-0008: Finalization gate](../atp/active/ATP-0008-finalization-gate.md) |

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
- all generated documents to match current facts, evidence metadata, and template versions;
- SHA-256 generation to cover the complete required set;
- native verification to pass for every listed file; and
- workflow and application versions to be available for the certificate.

`FAIL`, `BLOCKED`, `NOT VERIFIED`, or an N/A item without a reason prevents finalization.

## Resolve missing items

Open the track overview and follow each missing-item action. Common blockers include:

- a missing Suno project URL;
- no final release WAV or other configured final release role;
- a missing AI artwork original after AI artwork was declared;
- no selected subscription evidence for the production period;
- an unanswered conditional ownership or license question;
- a stale generated document after an input changed;
- an unprocessed visible AI disclosure required by project policy; or
- an integrity mismatch.

Do not enter a guessed historical value or attach an unrelated file merely to clear a blocker. A truthful `NOT VERIFIED` state is preferable to fabricated documentation.

The final release WAV and final artwork are each singular. If either changes, remove its current evidence entry before importing the replacement so generated documents and the certificate cannot refer to an ambiguous asset.

## Review artwork transparency

If the artwork is AI-generated or AI-assisted, review the original, any intermediate stages, the configured project transparency policy, disclosure text, and final output. Under the default policy, generate the visible local disclosure before choosing the final release artwork.

Confirm that:

- the AI original still exists unchanged;
- the output with disclosure is a separate file;
- the disclosure is visible and uses the configured text and placement;
- the disclosed output has `generated_disclosure` provenance, identifies the verified AI-original evidence as its source, records generator version `local-disclosure-v1`, and retains the exact configured text;
- the imported final artwork is exactly that locally generated disclosed output (the native gate compares its SHA-256); a manually imported edited image or unrelated unmarked final image is blocked;
- `AI_USAGE.md` and `artwork_process.md` identify the service, base image, human modifications, policy, result, text, and final output; and
- positive real-person, real-event, trademark, or logo answers have the configured factual note or evidence.

The content check records your answers. It does not decide legality.

## Generate current documents

Use the document-generation action after completing the relevant steps. Review the generated Markdown and text files for factual accuracy. The documents should state confirmed facts and applicable N/A reasons, not legal guarantees.

While the native service prepares, renders, and atomically writes the managed documents, the progress view shows the current phase, elapsed time, current relative path, and completed document count. These are live operation values rather than a simulated upload. The animated scene is only a visual companion; the final native result remains authoritative. Generated headings and guided values remain English even when the interface and progress view are German.

If an unmanaged document already exists at a managed destination, the application first shows the existing state. It writes managed content only after explicit confirmation and a backup below `.archive/`. Resolve any collision before continuing.

After changing a source answer or evidence selection, regenerate affected documents so the application no longer reports them as stale.

## Generate and verify SHA-256

Use the integrity step to generate `03_DOCUMENTATION/SHA256SUMS.txt`. The application hashes release files, Suno evidence, documentation, licenses, and artwork. It excludes `.archive/`, `.summary/`, the hash list itself, and `06_CERTIFICATE/`. Workspace `.suno-doc/` data sits outside the track root; a nested directory with that name inside a track is normal protected content.

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
06_CERTIFICATE/
├── DOCUMENTATION_CERTIFICATE.md
├── EVIDENCE_MANIFEST.json
└── CERTIFICATE_SHA256.txt
```

During finalization, the progress view distinguishes native gate validation, snapshot collection, transaction protection, certificate/manifest generation, certificate verification, the complete SHA-256 reread, and the final database commit. File names, byte counts, file counts, and elapsed time remain visible while the filesystem work runs outside the Tauri main thread.

After the verified native result is committed, a certificate summary opens automatically. It shows the certificate ID, track, artist, finalization time, workflow, verified integrity count, evidence count, blocking-deviation count, and final result. Close it to continue, or open the complete certificate view. For a still-valid finalized track, the same summary remains available through `Show certificate` in Finalize and `Open certificate summary` in the Certificate section.

Review the certificate ID, track, artist, workflow and application versions, timestamp, mandatory-step result, N/A reasons, evidence count, selected hashes, blocking-deviation result, and final status. Expected success status is `DOCUMENTATION COMPLETE`.

Review `EVIDENCE_MANIFEST.json` and confirm that file paths are relative to the track root. The manifest contains no local absolute workspace path. Each verified evidence object includes `provenance`; global copies retain their source record and coverage, while a local generated disclosure retains `derivedFromEvidenceId`, `generatorVersion`, and `generatedDisclosureText`. These fields are the portable lineage record and should agree with the evidence role and current artwork policy.

The certificate includes this mandatory meaning:

> This certificate confirms completion of the configured documentation workflow and integrity checks. It does not constitute governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.

## Verify certificate artifacts

From the track root, a system with `sha256sum` can independently check the certificate integrity list:

```sh
sha256sum -c 06_CERTIFICATE/CERTIFICATE_SHA256.txt
```

The certificate list covers at least the main SHA-256 list, evidence manifest, and certificate Markdown. It does not hash itself.

## Preserve the snapshot

Keep the complete track folder together. The folder is designed to remain reviewable without the SQLite index or application. A copy that omits evidence or generated documents is not the same finalized snapshot.

Do not edit a finalized file in place. If a release asset, evidence file, generated document, or other protected input changes outside the application, the next integrity check reports that the certificate no longer matches.

## Create a revision after a change

Every finalized track is shown as a read-only snapshot. Its workflow rail, tabs, Dashboard, Tracks, Workspace, and Settings remain navigable, and evidence previews plus non-mutating integrity verification remain available. The application does not repeatedly attempt to save a finalized form while navigating. Actions that would change fields, evidence, generated documents, hashes, or step status stay disabled until a revision exists.

Use `Create new revision and edit` from the overview, any workflow step (including Integrity and Finalize), or the certificate view when you intend to document a new snapshot. This action is available for both valid and invalid finalized certificates. Do not invalidate a valid certificate merely to make the revision action appear.

When the application reports `Documentation changed after finalization`, review the mismatch before proceeding. Then create the revision explicitly.

The application archives `revision.json`, the prior `03_DOCUMENTATION/SHA256SUMS.txt`, and the complete former certificate directory below `.archive/revisions/<revision-id>/`, then opens a new working revision. It can preserve this recovery record even when the live certificate was already damaged. Update the relevant facts or evidence, regenerate documents, apply artwork disclosure if required, regenerate and verify hashes, and pass the complete finalization gate again.

Tracks created by an older application version or recovered from an imported folder may not yet contain `.archive/revisions/`. Revision creation safely creates this managed parent before moving any live certificate artifact; users do not need to create the folder manually.

If the application or machine stops during certificate publication, reopening the workspace uses the matching `.archive/finalization-in-progress.json` marker to identify only that application transaction. A published set beside a non-finalized record is then moved to `.archive/recovery/<transaction-id>/certificate/` with recovery metadata. A stale marker beside an already finalized record is removed. Historical `06_CERTIFICATE/` files without this marker are left untouched and are not assumed to be a failed application finalization.

A new workflow version does not modify an older certificate. The application shows the finalized and current workflow versions and requires explicit reevaluation.

## Verification

Use a non-sensitive test track and record actual results only in the acceptance protocol. Verify at least one blocked attempt and one eligible attempt:

1. Leave one mandatory evidence role empty and confirm finalization is blocked with the exact missing item.
2. Complete every mandatory item, regenerate documents, and generate and verify hashes.
3. Finalize and inspect all three certificate artifacts.
4. Independently verify both SHA-256 lists where the platform tool is available.
5. Modify a protected disposable file and confirm certificate invalidation.
6. Create a revision and confirm that the previous certificate state remains archived.

Executed results and remaining manual checks are recorded in [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md), [ATP-0008](../atp/active/ATP-0008-finalization-gate.md), [ATP-0009](../atp/active/ATP-0009-certificate-generation.md), [ATP-0010](../atp/active/ATP-0010-certificate-invalidation-and-revision.md), and the [acceptance report](../dev/acceptance-report.md).

## Related documents

- [Getting started](getting-started.md)
- [Track documentation model](../def/track-documentation-model.md)
- [Workflow model](../def/workflow-model.md)
- [Persistence and recovery](../def/persistence.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-15 | Added finalization progress and the automatic, reusable certificate-summary dialog. | Project team |
| 2026-08-15 | Documented live document, SHA-256, and verification progress, including the immediate second hash pass and reduced-motion behavior. | Project team |
| 2026-08-15 | Documented automatic revision-parent repair for older and imported tracks. | Project team |
| 2026-08-15 | Clarified read-only finalized navigation and the directly available revision action. | Project team |
| 2026-08-13 | Added portable disclosure-lineage review and marker-scoped finalization recovery. | Project team |
| 2026-08-13 | Documented revision archive contents and interrupted-operation recovery. | Project team |
| 2026-08-13 | Added the finalization, certificate-verification, and revision guide. | Project team |
