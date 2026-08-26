<!-- AUTO-GENERATED:backlink START -->
[← Back](tools.md)
<!-- AUTO-GENERATED:backlink END -->
# Template-Projekte v1.0.0 release notes — historical reference

| Field | Value |
| --- | --- |
| Version | `1.0.0` |
| Status | Archived |
| Owner | Upstream Template-Projekte maintainers |
| Last review | 2026-08-25 |
| Validated upstream release commit | `a09c0b9998881e3dbbbd6292fdd22715b402bee8` |

## Evidence boundary

This managed path preserves the release-note context shipped by the v1.0.3 scaffold. It describes the first governed Template-Projekte release and its reusable quality, lifecycle, profile, and unsigned verification model. It is not a SunoDM release note and supplies no evidence that a SunoDM GitHub workflow, installer, signature, publication, or product acceptance run succeeded.

The upstream release supported web, cloud-backend, and Tauri profiles plus an optional PostgreSQL capability. SunoDM uses only the `desktop-local` profile with `frontend` and `tauri`; FastAPI, PostgreSQL, deployment containers, and the template publication workflow are not product capabilities.

## Relevant migration baseline

The reusable material subsequently included in Template-Projekte v1.0.3 provides:

- the central code-quality and architecture gate;
- deterministic template lifecycle state and local three-way update planning;
- profile-aware tooling and tests;
- documentation navigation checks;
- frontend test, lint, type, accessibility, and bundle-budget controls; and
- unsigned desktop validation patterns.

SunoDM integrates only the applicable product-shaped parts. Product identity, version `0.1.0`, SQLite and portable-file contracts, native command interface, third-party assets, and product acceptance history remain product-owned.

## Release and publication boundary

The original upstream release artifacts and workflow results belong to Template-Projekte. SunoDM's checked-in validation workflows have no publication job, signing credentials, notarization step, updater, or application-store integration. Their presence is configuration, not execution evidence. Current product release readiness is determined only from the SunoDM release model and actually recorded verification results.

## Related documents

- [Release and desktop packaging model](release-model.md)
- [Continuous integration](ci.md)
- [Code quality](../def/code-quality.md)
- [Template lifecycle](../def/template-lifecycle.md)
- [Upstream template final acceptance — historical reference](../dev/template-final-acceptance.md)
