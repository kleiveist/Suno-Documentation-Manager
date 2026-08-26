<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# SunoDM observability decision

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |
| Audience | Product architects, maintainers, and acceptance owners |
| Related ATP | [ATP-0013: End-to-end offline track workflow](../atp/active/ATP-0013-end-to-end-offline-workflow.md) |

## Purpose

This page records the product-specific observability decision required by the inherited template framework. Suno Documentation Manager is a local desktop application: normal use has no telemetry, tracking, remote log export, metrics exporter, tracing backend, account service, or mandatory network connection.

## Active decision

| Concern | SunoDM decision |
| --- | --- |
| Service objectives | No hosted service is operated by this repository. Desktop responsiveness, correctness, and recovery are verified through product tests and acceptance plans rather than a remote service objective. |
| Logs | Development commands and native failures write only local process output. Secrets, credentials, unrestricted file contents, and complete user workspaces must not be logged. |
| Metrics and traces | No runtime metrics or distributed traces are collected or exported. |
| Error reporting | Errors stay in the local application or development console. There is no automatic crash-report upload. |
| Correlation | Product operations use bounded product identifiers where needed; they do not introduce a remote telemetry identifier. |
| Dashboards and alerts | Not applicable because SunoDM operates no hosted product service. |
| Privacy | Workspace contents, track metadata, evidence, hashes, certificates, and local paths remain local unless the user explicitly invokes a separately documented external provider operation. |
| Cost and failure mode | There is no telemetry backend. Its absence cannot block application use. |

The explicitly user-started ACRCloud comparison and configured timestamp-authority request are product operations, not observability. Their bounded payloads, consent boundary, local evidence, credential handling, and non-blocking failure behavior are documented in the product architecture and workflow pages.

## Local diagnostic boundary

`python tools/control.py doctor`, `config doctor`, and `tauri doctor` report local setup and dependency problems. They must mask secrets and must not transmit the diagnostic result. The browser-only preview and the native desktop runtime do not expose template FastAPI health or readiness endpoints because the active `desktop-local` profile contains no backend.

## Change control

Adding telemetry, remote crash reporting, analytics, hosted dashboards, or a monitoring SDK changes the product privacy and network boundary. Such a change requires an explicit architecture decision, reviewed data classification and retention, user-facing disclosure where applicable, credential isolation, an outage policy, and product acceptance coverage before activation.

## Verification

Reviewers verify the `desktop-local` profile, the absence of an enabled telemetry adapter, and the product's network exclusions through the applicable quality, test, documentation, and acceptance gates. This page records the decision; it does not claim that a remote workflow or manual acceptance run has passed.

## Related documents

- [Application architecture](architecture.md)
- [Product architecture](product-architecture.md)
- [Runtime configuration](configuration.md)
- [Pre-release audio screening](pre-release-audio-screening.md)
- [Suno track workflow](workflow-model.md)
