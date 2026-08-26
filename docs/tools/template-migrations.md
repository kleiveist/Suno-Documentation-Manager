<!-- AUTO-GENERATED:backlink START -->
[← Back](tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Template migrations

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Audience | Template maintainers and migration authors |
| Related evidence | [Template v1.0.3 migration plan](../dev/migrations/template-v1.0.3/migration-plan.md) |

## Purpose

This page defines the controlled registry for structural template changes that an ordinary three-way file merge cannot express safely, such as a known path move or a versioned configuration-key transformation.

## Boundary

A registered structural migration operates only on template-managed source structure in an isolated staging tree. It is not permission to modify SunoDM business data, SQLite, workspaces, tracks, evidence, hashes, certificates, manifests, timestamp records, audio-screening formats, or user files. Those product contracts require their own reviewed migration and explicit data-or-format approval.

The registry also excludes arbitrary Python or shell hooks, network access, Git history mutation, automatic profile changes, deployment activity, and external service calls.

## Migration contract

Each migration declares a stable unique ID, supported source and target revisions, deterministic order, preconditions, bounded declarative operations, postconditions, and idempotence behavior. An ID is recorded in lifecycle state only after the complete staged update and final verification succeed.

Supported operation families are constrained path moves/copies/deletions, bounded key or structured-document transformations, deterministic text transformations, and non-mutating report notices. Every path is validated against traversal, absolute paths, drive-qualified paths, external symbolic links, protected runtime areas, and undeclared product-owned destinations.

## Execution and rollback

```text
resolve exact target
  -> reconstruct BASE and INCOMING
  -> select and validate migrations
  -> copy product to isolated staging
  -> apply migrations and three-way operations
  -> verify postconditions and staged product
  -> transactionally replace product paths
  -> write lifecycle state last
```

A failed precondition, operation, postcondition, merge, or final verification blocks the entire update. Product paths and both lifecycle files are restored; the failed migration ID remains absent. An ignored diagnostic report may remain.

## Authoring and verification

Migration authors use synthetic temporary repositories, cover success, repetition, conflict, failure, and rollback, and prove that product-owned destinations and protected paths cannot be overwritten. A production workspace is never the first migration fixture.

```sh
python tools/control.py quality
python tools/control.py test --suite tools
python tools/control.py docs check
```

The v1.0.3 lifecycle registry contains no SunoDM product-data migration and does not authorize one.

## Related documents

- [Template lifecycle](../def/template-lifecycle.md)
- [Code quality](../def/code-quality.md)
- [Project profiles](../def/project-profiles.md)
- [Tooling guide](tooling.md)
- [Migration audit](../dev/migrations/template-v1.0.3/audit.md)
