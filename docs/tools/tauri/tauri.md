<!-- AUTO-GENERATED:backlink START -->
[← Back](../tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Tauri desktop development

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-16 |
| Audience | Desktop developers and package operators |
| Related ATP | [ATP-0013: End-to-end offline workflow](../../atp/active/ATP-0013-end-to-end-offline-workflow.md) |

## Purpose

This page defines the supported native development and packaging entry points for the active `desktop-local` Suno Documentation Manager product.

## Scope

### Included

- Rust and platform prerequisites;
- complete Tauri development startup;
- native verification and local unsigned packaging; and
- the Tauri command and permission boundary.

### Excluded

- signing, notarization, publication, and automatic updates;
- backend, container, or cloud deployment; and
- a standalone browser preview as a complete product runtime.

## Prerequisites

The Rust crate declares MSRV 1.88 through `rust-version = "1.88"` in `src-tauri/Cargo.toml`. Use a stable Rust toolchain at version 1.88 or newer, Node.js with the locked frontend dependencies, and the Tauri 2 platform prerequisites reported by the native doctor.

From the repository root:

```sh
python tools/control.py doctor
python tools/control.py tauri doctor
python tools/control.py install
```

Normal product runtime is offline. Dependency installation and platform package setup can require network access.

## Development

Run the complete desktop product with:

```sh
python tools/control.py tauri run --foreground
```

The standalone Vite browser preview is useful only for presentation work. It cannot execute the native workspace, SQLite, evidence, disclosure, hash, or certificate operations.

## Verification

```sh
python tools/control.py test --suite tauri
python tools/control.py build desktop --dry-run
python tools/control.py build desktop --target linux --bundles deb,rpm
```

Use `python tools/control.py build desktop --help` for target and bundle options. Packages produced by the ordinary local path are unsigned verification artifacts unless a separate reviewed signing process is configured.

## Artifact names

`src-tauri/tauri.conf.json` defines the full product label, `Suno Documentation Manager`, through `productName` and the main-window title. The separate `mainBinaryName` value `sunodm` is the executable name and the base used by product-owned archives.

The executable is `sunodm` or `sunodm.exe`. Product-owned collected outputs use stable names such as `~/Applications/sunodm.AppImage`, `sunodm-web.zip`, and `sunodm-windows-portable.zip`. Standard Tauri DEB, RPM, AppImage, MSI, NSIS/EXE, DMG, and application-bundle filenames may instead derive from `productName` and append format-specific version, architecture, or installer suffixes; the copy and Linux evidence tools retain those produced package names.

The Python build tools expose the short executable/archive base as `tools.tauri.paths.APP_ARTIFACT_NAME`. Project generation applies the generated project slug to the binary identity while preserving its full display name in `productName` and the window title. If the source configuration pins a WiX UpgradeCode, generation deterministically replaces it with the code derived from the target binary so independent products cannot share an MSI upgrade line.

Tauri derives a default WiX UpgradeCode from `productName`. Because the lifecycle identity correction changes `productName` from the former shorthand to the display name, `bundle.windows.wix.upgradeCode` explicitly pins the existing `sunodm`-derived value `54c9e875-02fa-5312-b9c4-14f17c9e3c61`. It must remain unchanged across Windows MSI updates so an installed version is upgraded rather than duplicated.

## Security boundary

The default Tauri capability grants `core:default` only. Product filesystem work is implemented behind named typed Rust commands; the webview has no broad filesystem allowlist or raw SQL interface. The detailed version 0.1 path-race boundary is documented in [Product architecture](../../def/product-architecture.md#version-01-threat-model-boundary).

## Related documents

- [Tooling guide](../tooling.md)
- [Release model](../release-model.md)
- [Application architecture](../../def/architecture.md)
- [Getting started](../../usr/getting-started.md)

<!-- AUTO-GENERATED:docs-index START -->

## 📄 Pages
- ⏭️ (no pages)

<!-- AUTO-GENERATED:docs-index END -->
