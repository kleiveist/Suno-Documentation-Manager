<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Upstream template lifecycle acceptance — historical reference

| Field | Value |
| --- | --- |
| Status | Archived |
| Owner | Upstream Template-Projekte maintainers |
| Last review | 2026-08-25 |
| Audience | SunoDM developers, reviewers, and acceptance owners |
| Source basis | Template-Projekte `v1.0.3` at `8f4299193b5afe8545dda55208baf3adec23a68a` |

## Evidence boundary

The upstream template recorded acceptance of its reusable lifecycle implementation before this pilot. Those template runs, profile matrices, database checks, native builds, workflow run identifiers, release commit, and readiness decision are historical upstream evidence only. They do not prove that the SunoDM migration, product tests, desktop bundles, or remote workflows passed.

The upstream completed lifecycle ATP is deliberately not copied into SunoDM because `ATP-0001` already identifies the product's workspace acceptance protocol. Reusing that identifier would corrupt product history and create a false acceptance relationship.

## Upstream scope represented by this path

The inherited acceptance covered deterministic provenance, legacy audit and adoption, three-way planning, structural migrations, transactional update and rollback, identity and version preservation, local source resolution, path and secret boundaries, CLI and report behavior, generated profile checks, documentation navigation, and the template's own CI and release gates.

## SunoDM disposition

For this repository, every lifecycle and product result starts from its own executed evidence. The technical migration audit, ownership decisions, block results, compatibility checks, adoption preview, and final lifecycle verification are recorded under [Template v1.0.3 migrations](migrations/template-v1.0.3/template-v1.0.3.md). Remote GitHub Actions, cross-platform desktop builds, signing, publication, and any unexecuted manual step remain `NOT RUN` or `NOT APPLICABLE`; the existence of a workflow file is not passing evidence.

This historical reference cannot change a migration item to `PASS`, cannot authorize a product data or format change, and cannot replace Gate C, adoption approval, or the product ATP process.

## Related documents

- [Template lifecycle](../def/template-lifecycle.md)
- [Template migrations](../tools/template-migrations.md)
- [Upstream template final acceptance — historical reference](template-final-acceptance.md)
- [Template v1.0.3 audit](migrations/template-v1.0.3/audit.md)
- [Product ATP workflow](../atp/README.md)
