<!-- AUTO-GENERATED:backlink START -->
[← Back](README.md)
<!-- AUTO-GENERATED:backlink END -->
# Third-party notices

| Field | Value |
| --- | --- |
| Status | Active inventory; distribution readiness is not yet approved |
| Owner | Blobbite.com |
| Last review | 2026-08-25 |
| Audience | Users, redistributors, release operators, and contributors |
| Related ATP | N/A — repository-wide licensing boundary |

## License boundary

The SunoDM project license does not replace or relicense third-party components. Each component remains subject to its own license terms, notices, source-availability obligations, and other distribution conditions. The root `LICENSE` and `NOTICE` govern SunoDM-authored material only to the extent stated there.

## Bundled components

### Chromaprint and fpcalc 1.6.1

SunoDM bundles target-specific `fpcalc` executables from Chromaprint 1.6.1. Chromaprint's own source is distributed under the MIT License, while the upstream binary distribution states that included FFmpeg portions are under the GNU Lesser General Public License version 2.1 and that the license of the external FFT library must also be considered.

Included notices and provenance:

- `src-tauri/binaries/CHROMAPRINT-1.6.1-PROVENANCE.md`
- `src-tauri/binaries/CHROMAPRINT-LICENSE.md`
- `src-tauri/binaries/LGPL-2.1.txt`

Upstream: <https://github.com/acoustid/chromaprint/releases/tag/v1.6.1>

The current provenance records the downloaded archives and the five bundled executable hashes. It does not yet identify a complete reproducible build bill of materials, the exact included FFmpeg and FFT components for every target, or a reviewed corresponding-source and relinking delivery mechanism. Distribution of these sidecars therefore remains **NOT READY** pending that review.

### DejaVu Fonts 2.37

SunoDM bundles DejaVu font files for deterministic certificate PDF rendering. The complete font license shipped with the source is:

- `src-tauri/assets/fonts/LICENSE-DejaVu.txt`

The font license and the bundled font bytes must remain together in every applicable desktop distribution.

### sigstore-tsa 0.10.0

SunoDM vendors `sigstore-tsa` 0.10.0 from the upstream `prefix-dev/sigstore-rust` project and applies the verification changes documented by the product. The vendored code and local patch are distributed under Apache License 2.0.

Included notices:

- `src-tauri/vendor/sigstore-tsa/LICENSE`
- `src-tauri/vendor/sigstore-tsa/README-SUNODM.md`

## Package-manager dependencies

Rust and npm dependencies are fixed by `src-tauri/Cargo.lock` and `frontend/package-lock.json`. Those lockfiles identify resolved packages but are not, by themselves, a complete binary-distribution license notice. Release validation generates an SPDX dependency SBOM; a release operator must review that exact candidate's licenses, notices, and source obligations before distribution.

## Release condition

This inventory improves source transparency but is not a distribution approval or legal opinion. No release may be represented as license-complete until the fpcalc build/source review and a lockfile-based third-party notice review are complete and the packaged resources have been inspected on every target.
