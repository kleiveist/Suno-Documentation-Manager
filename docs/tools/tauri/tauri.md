<!-- AUTO-GENERATED:backlink START -->
[← Back](../tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Tauri desktop development

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-13 |
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
