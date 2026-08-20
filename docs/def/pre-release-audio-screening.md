<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Pre-release audio screening

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Product team |
| Last review | 2026-08-20 |
| Audience | Product developers, reviewers, and operators |
| Related ATP | [ATP-0017: Pre-release audio screening](../atp/active/ATP-0017-pre-release-audio-screening.md) |

## Purpose

This definition describes the optional, two-stage technical audio screening that is available before a track is finalized. It records reproducible local fingerprint facts and, only after an explicit user action, an optional ACRCloud identification result. It is a comparison aid, not a rights, ownership, release-clearance, infringement, or legal determination.

## Scope and trigger boundary

Stage 1 runs locally when the authoritative `release_wav` evidence is imported or replaced in an editable track. SunoDM runs its bundled, pinned Chromaprint `fpcalc` engine against that managed evidence and writes a portable record below:

```text
03_DOCUMENTATION/AUDIO_SCREENING/
├── LOCAL_FINGERPRINT.json
├── LOCAL_FINGERPRINT.sha256
└── AUDIO_SCREENING.md
```

The local record binds the engine and version, algorithm, source Evidence ID, track-relative source path, source SHA-256 and size, measured duration, generation time, and the full technical fingerprint. `LOCAL_FINGERPRINT.sha256` is its detached digest: the JSON cannot safely contain a hash of its own final bytes. The full fingerprint remains in that local record only; summaries, the certificate, PDF, manifest, and frontend view never render it. These files are ordinary phase-one documentation files and are included by `03_DOCUMENTATION/SHA256SUMS.txt`.

Stage 2 is never automatic. It may run only when the user explicitly chooses the external screening action in Step 07, after the workspace owner has enabled and configured ACRCloud under Settings. The workspace owner chooses an ACRCloud screening intensity and either calculates it from the actual track duration or from a fixed reference duration. The run turns that target into a deterministic set of distinct bounded samples made from the release evidence, then creates a structured result plus, when safe JSON responses exist, provider-response archive data:

```text
03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_SCREENING.json
03_DOCUMENTATION/AUDIO_SCREENING/ACRCLOUD_RESPONSE.json (or a versioned multi-sample response archive)
```

The external request is not part of import, replacement, document generation, integrity generation or verification, app startup, reopening a workspace, workflow evaluation, or finalization. It cannot block finalization. A missing configuration is represented as `SKIPPED_NOT_CONFIGURED`; it is not a match result.

## Data and privacy boundary

The local screening state is associated with the track and carries source identity and freshness information. Replacement or removal of the authoritative release evidence makes prior state stale; a subsequent local run replaces it with a record bound to the new source bytes. A revision archives the completed prior screening documentation together with the prior phase-one snapshot and begins a new editable state rather than transferring a provider result to a different revision.

ACRCloud credentials are workspace-local secret configuration. The access key and access secret are write-only from the UI, never enter a track record, portable documentation, manifest, certificate, PDF, diagnostic message, or log, and are not returned by typed DTOs. The response archive contains only the provider response and factual request context that is safe for the portable record; it contains no request signature or credential.

No Chromaprint fingerprint is uploaded to ACRCloud. The provider receives only the bounded audio samples required by its Identification API. The app uses HTTPS to the configured ACRCloud host and a fixed `/v1/identify` path, generates the documented HMAC-SHA1 request signature locally, applies byte, response, and timeout bounds, and reports controlled provider/configuration/transport failures without falsely reporting a match.

## Multi-sample intensity and planning

The screening-intensity setting is a percentage. With dynamic calculation, the requested coverage is the authoritative measured track duration multiplied by that percentage. With a fixed reference duration, the same calculation uses the configured reference duration and is then capped at the actual track duration. The setting is an explicit sampling target, not a claim that a track has undergone a complete rights or release-clearance review.

Each ACRCloud request contains at most 12 seconds of audio. A run can make at most 25 requests and can therefore submit at most 300 seconds of external audio samples. The planner rounds the requested target up to the request needed for the final partial 12-second interval, then caps it by the track's non-overlapping capacity and the hard request limit. A track shorter than 12 seconds can use one correspondingly shorter sample. If another unique interval is not available, the planner reduces the request count instead of uploading a duplicate or overlapping interval.

Sample offsets are deterministic for the same release bytes, measured duration, intensity, and configuration. They are distributed across the complete track rather than clustered at its beginning. Every actual sample record contains its ordinal, start offset, end offset, duration, provider status/result, and response-artifact linkage where a safe response was retained. The aggregate record captures the requested intensity and mode, target duration, planned and executed request counts, unique/overlapping/duplicate sample counters, total distinct sampled duration, measured coverage, provider status, check time, and the authoritative release SHA-256. Successful plans always report zero duplicate and zero overlapping samples.

## Result semantics

`FINGERPRINT_GENERATED` means that the bundled local engine produced a fingerprint for the current authoritative release bytes. It does not imply uniqueness, authorship, ownership, or clearance.

`NO_MATCH_DETECTED` and `MATCH_DETECTED` are provider-response facts from a manually requested ACRCloud check. `MATCH_DETECTED` records only concise provider-supplied fields (title, artist list, album, ISRC, ACRID, and score when returned), limited to a small display list. In a multi-sample run, every request keeps its own factual result and a match in any sample remains visible in the aggregate result. It is not a conclusion that the current release infringes or is cleared. `PROVIDER_UNAVAILABLE`, `AUTHENTICATION_FAILED`, `CONFIGURATION_INVALID`, `ENGINE_UNAVAILABLE`, `UNSUPPORTED_FORMAT`, `PROCESSING_FAILED`, `STALE`, and `NOT_RUN` are non-positive technical statuses.

## Snapshot and integrity behavior

For a new finalization, the sanitized `audioScreening` section captures local/external status, engine/provider identifiers, source linkage, intensity/mode/coverage facts, per-sample offsets and results, archive paths and hashes, and concise matches, but excludes the raw fingerprint, raw provider response, and all secrets. The Markdown certificate and both PDF/A-2b root PDFs render the same concise K.2 multi-sample summary and the explicit technical/no-legal-claims boundary.

Existing finalized certificates, manifests, PDFs, hash lists, and archived revisions are not regenerated or backfilled. A newer renderer only applies to new documentation/finalization snapshots. Historical verification hashes the existing published bytes.

## Operational rules

- A bundled target binary is checked before use; an unsupported or unavailable platform returns `ENGINE_UNAVAILABLE` instead of falling back to a system path or a substitute algorithm.
- The source is the managed release evidence selected by `EvidenceRole::ReleaseWav`, not a filename guess and not a SHA-256 substitute for audio content.
- Files, response bytes, process output, duration, sample size, and request time are bounded. Each request is limited to 12 seconds, a run is limited to 25 requests / 300 seconds, and every planned interval is checked for containment, uniqueness, and non-overlap before upload. The application uses no shell and no user-controlled executable or endpoint path.
- External screening is a deliberate best-effort action. It has no background retries and no hidden connection attempt.
- `AUDIO_SCREENING.md`, `LOCAL_FINGERPRINT.json`, `LOCAL_FINGERPRINT.sha256`, and, when present, `ACRCLOUD_SCREENING.json` plus `ACRCLOUD_RESPONSE.json` are integrity-protected portable documents; `.archive/` copies are excluded from the current phase-one hash list.

## Related documents

- [Product architecture](product-architecture.md)
- [Persistence and recovery](persistence.md)
- [Workflow model](workflow-model.md)
- [Finalizing a track](../usr/finalizing-a-track.md)
- [ATP-0017: Pre-release audio screening](../atp/active/ATP-0017-pre-release-audio-screening.md)
