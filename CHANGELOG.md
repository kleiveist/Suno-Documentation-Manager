<!-- AUTO-GENERATED:backlink START -->
[← Back](README.md)
<!-- AUTO-GENERATED:backlink END -->
# Changelog

## 2026-08-20

- Accepted the exact alternate Suno WAV metadata marker `made with suno` alongside `made with suno studio`, while retaining the single-marker, single-`created`, single-`id`, RFC 3339, UUID, ambiguity, and bounded-text checks. This is structural evidence extraction, not provider authentication; existing finalized snapshots remain byte-identical and are analyzed only in an explicit new revision.
- Raised the current Suno workflow to `1.9`, document templates to `1.10`, evidence-manifest schema to `7`, and Markdown/PDF certificate format to `6.0`; existing finalized and archived bytes remain unchanged.
- Added an optional post-commit automatic timestamp phase fixed to the exact SHA-256 of finalized `EVIDENCE_MANIFEST.json`. RFC-3161 `VERIFIED` now requires a matching SHA-256 message imprint, fresh request/response nonce, returned policy OID and requested-policy match when configured, CMS signature, a critical sole `id-kp-timeStamping` EKU, signer validity at `genTime`, and a chain to explicitly configured TSA trust anchors. Provider failure never rolls back the phase-one certificate.
- Kept manual/legacy timestamp attachments as explicit hash-comparison evidence and initial OpenTimestamps proofs as `ATTACHED`; neither path is promoted to RFC-3161 cryptographic verification. Sidecar reload now derives positive status only from the exact registered/published bytes, immutable snapshot binding, anchor, provider metadata, and the complete current verification predicate.
- Added a presentation-only automatic-consistency result with `INFO`, `WARNING`, and `BLOCKING` findings. Informational observations keep `PASS`, unresolved non-blocking findings produce `PASS WITH WARNINGS`, and authoritative blockers remain `BLOCKED`; the established workflow and finalization gates are unchanged.
- Vendored and narrowly hardened `sigstore-tsa` 0.10.0 from `prefix-dev/sigstore-rust` commit `2501a347c5c858bb91feb96f40f8eb67f06d6418` for declared-algorithm CMS verification, RSA PKCS#1/RSA-PSS support, and strict timestamping EKU checks.
- Changed new German/English certificates and timestamp addendum PDFs to deterministic PDF/A-2b with XMP identification, a CMYK FOGRA39 output intent, and the complete DejaVu 2.37 Sans/Mono regular and bold font programs embedded under the DejaVu Fonts License. Manifest schema 7 records the archive profile, font names, font hashes, version, license, and output intent.
- Replaced Suno Generation Text Field multi-selection with exactly one `STRUCTURE_ONLY`, `VOCAL_LYRICS_ONLY`, `MIXED`, `EMPTY`, or `OTHER` value and replaced Boolean Vocal Intent with independent `VOCAL`, `INSTRUMENTAL`, or `UNSPECIFIED` intent. Vocal lyrics plus structure instructions are `MIXED`; actual audio never changes intent.
- Required every AI-generated or AI-assisted artwork to record Artwork Disclosure as explicit `YES` or `NO`; `YES` requires disclosure text and verified local lineage, while `NO` is retained as deliberate non-application.
- Added informational `BYTE-IDENTICAL / SHA-256 MATCH` reporting for equal human-edited/final artwork, clarified that import timestamps do not prove creation/editing chronology, and standardized only new artwork filenames to an ASCII-uppercase track stem plus `_AI_ORIGINAL`, `_AI_EDITED`, `_HUMAN_EDITED`, or `_FINAL`. Existing and finalized evidence is never renamed.

## 2026-08-17

- Hardened external-timestamp sidecar format v1 around a durable stage → SQLite registration → live publication sequence. The complete stage and parent are synchronized before registration, and live-parent synchronization precedes a compensating database rollback. Startup publishes matching registered pending stages, removes abandoned unregistered stages, and rejects unexpected unregistered live sidecars instead of adopting user-confirmed metadata.
- Made `TIMESTAMP_RECORD.json` an exact immutable publication record with `integrityVerifiedAtPublication` and pinned Markdown/PDF SHA-256 values while keeping current `integrityVerified` and issues as load-time presentation state. Verification rejects injected runtime/trust claims even with a renewed hash list, hashes published bytes without re-rendering them, and requires archived `revision.json.previous_certificate.certificateId` to match the sidecar before reporting archive integrity without affecting the base certificate.
- Enforced the Terms availability invariant at the native update, workflow-consistency, Markdown-certificate, and PDF-certificate boundaries: verified local Terms evidence cannot coexist with an unavailable claim.
- Completed ATP-0016: all user Tests 01–18 passed their recorded automated or manual expectations, including the 120-dpi A–L PDF review and an independently reproduced retained portable-track review. No legal assessment or external timestamp-authority trust/qualification validation is claimed.

- Raised the Suno workflow to `1.7`, document templates to `1.8`, SQLite schema to `5`, evidence-manifest schema to `5`, and Markdown/PDF certificate format to `5.0`.
- Replaced the current Suno-field multi-classification with one canonical Content Classification (`STRUCTURE_ONLY`, `VOCAL_LYRICS_ONLY`, `MIXED`, `EMPTY`, or `OTHER`) and made Vocal Intent an independent explicit `VOCAL`/`INSTRUMENTAL`/`UNSPECIFIED` value. Legacy arrays remain readable and migrate only on an explicit upgrade/revision when unambiguous; neither intent nor final-audio outcome is inferred or cross-blocked.
- Kept historical `sunoPlanAtCreation` data in a separate legacy field; it never populates the new plan-at-generation fact, which remains `NOT DOCUMENTED` until explicitly confirmed.
- Expanded Final Suno Generation into separate date, Suno ID, project URL, model, plan-at-generation, metadata-origin, download/export, and release/export hash-comparison facts.
- Kept subscription coverage as a technical date-interval comparison and required document title, provider/source, and retrieval date for commercial Terms evidence; optional context remains factual and offline.
- Split AI transparency into audio and artwork assessments, preserved `YES`, `NO`, `N/A`, and `NOT DOCUMENTED` semantics, and made an undocumented disclosure status block commercial generative-AI finalization.
- Added optional post-finalization external-timestamp records and deterministic PDF/Markdown addenda bound to the selected certificate revision and an existing stable hash anchor. Custom `Other` anchors must be unchanged entries in the verified phase-one hash list. Published sidecars are independently reverified and expose their own integrity status; claimed and locally calculated hashes remain distinct, mismatches are visible, the base certificate is unchanged, and no legal timestamp qualification is inferred.
- Clarified that workflow `PASS` and certificate `DOCUMENTATION COMPLETE` describe configured documentation completion only and never rights clearance, legal compliance, evidentiary weight, or governmental certification.

## 2026-08-16

- Simplified global Suno terms registration to one direct native PDF selection with no metadata form; only signature-checked PDF files are accepted and propagated to editable projects.
- Excluded every leading-dot workspace folder (for example `.archive`, `.cache`, and `.git`) from album listing, track discovery, identity recovery, and library rendering, including previously indexed hidden paths.
- Moved Suno terms/rights import to Settings as global evidence, automatically copied into new and editable projects while finalized snapshots remain untouched; raised SQLite schema to 4.
- Removed project/version ID, final generation ID, and final-generation time from current workflow inputs and generated certificate/documents; compatibility fields in existing stored records remain non-blocking.

- Raised the Suno workflow to 1.3, document templates to 1.5, SQLite schema to 4, evidence manifest schema to 2, and certificate format to 3.0.
- Added concrete final-generation metadata, instrumental/Lyrics consistency, original-filename deviation confirmation, and commercial subscription coverage checks.
- Added local `suno_terms_rights` and `external_timestamp` evidence roles with persisted provenance, file facts, and role-specific compatibility metadata.
- Reworked PDF and Markdown certificates into the A–J technical evidence structure with statement-origin labels, earlier-revision references, deterministic PDF trailer IDs, and explicit non-legal scope.
- Preserved finalized-snapshot immutability, revision archival, relative paths, and SHA-256 integrity behavior.
