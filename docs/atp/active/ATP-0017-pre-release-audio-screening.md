<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0017: Pre-release audio screening

| Field | Value |
| --- | --- |
| Status | Planned execution |
| Owner | Product team |
| Last review | 2026-08-20 |
| Audience | Acceptance owners and product developers |
| Related definition | [Pre-release audio screening](../../def/pre-release-audio-screening.md) |

## Objective

Validate that SunoDM records a real local Chromaprint result for the authoritative release evidence, offers ACRCloud only as an explicit optional external check, preserves portable integrity artifacts, and makes no legal or ownership conclusion.

## Acceptance matrix

| ID | Scenario | Expected result | Automated evidence | Status |
| --- | --- | --- | --- | --- |
| AS-01 | Import a supported release audio file in an editable track | Bundled `fpcalc` runs against the managed `release_wav`; `LOCAL_FINGERPRINT.json`, its detached SHA-256, and `AUDIO_SCREENING.md` bind source Evidence ID/path/SHA-256, engine version, algorithm, duration, and generated time | Rust integration test creates a real short WAV and asserts a nonempty local fingerprint record and source binding | Planned |
| AS-02 | Replace the authoritative release file | Old screening state is stale/replaced; the new local record has the replacement SHA-256 and does not claim the old bytes | Rust replacement test plus track-state assertions | Planned |
| AS-03 | Use a target without a verified bundled engine | Controlled `ENGINE_UNAVAILABLE`; no system executable lookup and no synthetic fingerprint | Unit test injects unavailable runner | Planned |
| AS-04 | Generate documents and hashes after a local run | Documentation freshness includes screening state; all present `03_DOCUMENTATION/AUDIO_SCREENING/*` artifacts occur in `SHA256SUMS.txt` | Rust document/integrity test | Planned |
| AS-05 | Finalize a new current record | Manifest schema 7 and certificate/PDF format 6.0 expose a concise K.2 summary, source binding, artifact hashes, and technical-only disclaimer in the deterministic PDF/A-2b certificate set; no raw fingerprint or secret appears | Rust manifest/Markdown/PDF extraction and forbidden-content tests | Planned |
| AS-06 | Open an old finalized snapshot | Existing certificate, manifest, PDF, and hashes remain byte-identical; no audio-screening backfill occurs | Rust historical fixture verification | Planned |
| AS-07 | Leave ACRCloud disabled or unconfigured | Step 09 explains configuration; the external state is non-positive (`SKIPPED_NOT_CONFIGURED`/configuration state), and finalization remains available | Native plus frontend workflow tests | Planned |
| AS-08 | Configure valid credentials and select external screening | Exactly one explicit HTTPS request uses a bounded audio sample; request contains no Chromaprint fingerprint; the structured result and safe response archive contain no credentials/signature | Adapter request-capture test | Planned |
| AS-09 | Provider returns no match, a match, authentication failure, or unavailable response | Status and concise response facts are accurate; failures never become positive match claims | Adapter parser and UI state tests | Planned |
| AS-10 | Create a revision after screening | Previous screening artifacts remain with archived snapshot; new editable revision has no transferred external provider conclusion and reruns/awaits local screening against its own source | Revision integration test | Planned |
| AS-11 | Inspect source and output for prohibited claims/secrets | No access key, access secret, request signature, raw fingerprint, `COPYRIGHT[_]SAFE`, `INFRINGEMENT[_]FREE`, or `LEGAL[_]SAFE` occurs in user-visible portable artifacts | `rg` negative scan plus Rust serialization tests | Planned |

## Required commands

```sh
python tools/control.py doctor
python tools/control.py tauri doctor
python tools/control.py test --suite all --report
python tools/control.py build web
python tools/control.py build desktop
python tools/control.py docs index --dry-run
python tools/control.py release check
rg -n "COPYRIGHT[_]SAFE|INFRINGEMENT[_]FREE|LEGAL[_]SAFE" frontend src-tauri docs workflows
```

## Evidence recording rule

Acceptance execution must identify the build, platform, bundled Chromaprint binary/version, and ACRCloud adapter fixture. Do not mark a provider integration PASS from a live account response alone; retain deterministic request/response fixtures with credentials removed. If a host cannot build the desktop target because required system libraries are absent, record that environmental limitation separately from unit and frontend results.

## Related documents

- [Pre-release audio screening](../../def/pre-release-audio-screening.md)
- [Product architecture](../../def/product-architecture.md)
- [Finalizing a track](../../usr/finalizing-a-track.md)
