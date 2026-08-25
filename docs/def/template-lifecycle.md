<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Template lifecycle

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Audience | Template and product maintainers |
| Migration evidence | [Template v1.0.3 audit](../dev/migrations/template-v1.0.3/audit.md) |

## Purpose

This document defines how SunoDM records template provenance, compares itself with another trusted local template revision, and applies template changes without taking ownership of product code, identity, version, data, or portable formats.

## Scope

The lifecycle supports deterministic state and baseline manifests, local audit and adoption, BASE/LOCAL/INCOMING planning, conflict detection, declarative structural migrations, transactional apply, rollback, and verification. It does not fetch remote history, create commits or tags, publish releases, resolve conflicts automatically, change profiles silently, or migrate product databases and user data.

Template and product versions remain independent:

| Value | Location | Meaning |
| --- | --- | --- |
| Product version | `VERSION` and enabled component mirrors | SunoDM release version |
| Template version | `.template/state.toml` source metadata | Human-readable template release |
| Template commit | `.template/state.toml` source metadata | Exact installed template provenance |

A lifecycle update never runs product version synchronization implicitly.

## Managed state

After separately approved adoption, a managed product commits only these lifecycle records:

```text
.template/
├── state.toml
└── baseline.json
```

The state records the canonical template identity and commit, provenance, profile and resolved features, product identity, baseline digest, and applied structural migration IDs. The baseline contains sorted path metadata and SHA-256 values, not file contents. Neither file may contain absolute local paths, timestamps, secrets, credentials, database content, or user data.

The baseline defines ownership dynamically:

```text
path in baseline manifest      -> template-managed baseline path
path absent from the manifest -> product-owned path
```

Runtime workspaces, `.env`, local databases, dependencies, build output, reports, caches, credentials, and detected user data remain protected regardless of manifest membership.

## Commands

All lifecycle commands are local and use an explicitly trusted template checkout:

```sh
python tools/control.py template status
python tools/control.py template audit --help
python tools/control.py template adopt --help
python tools/control.py template plan --help
python tools/control.py template update --help
python tools/control.py template verify
```

`status`, `audit`, `adopt` without `--apply`, `plan`, and `update` without `--apply` are read-only. Adoption writes lifecycle metadata only after explicit `--apply` and a clean product worktree. Update apply refuses unresolved conflicts and revalidates the exact source commit before writing.

## Three-way and transaction rules

Ordinary file changes compare the installed `BASE`, current product `LOCAL`, and target-template `INCOMING` versions. Unchanged local content can take an incoming update; independent local edits are preserved or merged; overlapping edits and binary dual changes are conflicts. A single unresolved conflict blocks the complete apply rather than permitting a partial subset.

Apply builds and verifies an isolated staging tree before replacing product paths. Lifecycle state is written last. A failure restores every changed product and lifecycle path. Reports remain under the ignored lifecycle report directory and contain relative paths only.

## SunoDM pilot boundary

The technical migration and lifecycle adoption are intentionally separate changes. Adoption is not a product migration and does not certify data or format compatibility. Any change to SQLite, workspace or track paths, evidence semantics, hashes, certificate or manifest formats, timestamp records, audio-screening formats, or the typed native command contract requires the product's separate data-or-format approval gate.

The current pilot source and ownership decisions are recorded under [Template v1.0.3 migrations](../dev/migrations/template-v1.0.3/template-v1.0.3.md). Implementation or an inherited template acceptance record is not SunoDM acceptance evidence.

## Verification

```sh
python tools/control.py template status
python tools/control.py template verify
python tools/control.py quality
python tools/control.py test --suite tools
python tools/control.py docs check
python tools/control.py version check
```

Only commands actually executed with their real result belong in migration or acceptance evidence.

## Related documents

- [Template migrations](../tools/template-migrations.md)
- [Tooling guide](../tools/tooling.md)
- [Code quality](code-quality.md)
- [Migration plan](../dev/migrations/template-v1.0.3/migration-plan.md)
- [Ownership matrix](../dev/migrations/template-v1.0.3/ownership-matrix.md)
