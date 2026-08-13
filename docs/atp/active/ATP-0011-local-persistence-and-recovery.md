<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0011: Local persistence and recovery

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-13 |
| Executed | 2026-08-13 — partial automated execution |
| Requirement | [`REQ-PER-002` through `REQ-PER-004`](../../def/persistence.md#requirements-and-atp-mapping), [`REQ-ARC-004`, `REQ-ARC-005`](../../def/product-architecture.md#product-requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; unversioned source tree; Linux package digests in the central report |
| Environment | Linux `7.0.8-1-cachyos` `x86_64`; in-memory SQLite fixtures and static command/schema review |

## Purpose

This plan verifies SQLite ownership, transactional migrations, restart persistence, global-evidence handling, and honest recovery from portable track folders after index loss.

## Objective

Accept local persistence when Rust alone owns SQLite, supported migrations are transactional, failed operations remain recoverable, and reindexing reconstructs evidenced facts while preserving unknown historical values.

## Scope

### Included

- connection/schema initialization and reopen;
- absence of arbitrary frontend SQL;
- supported and failing migration behavior;
- file/index partial-failure recovery;
- index-loss scan and finalized-track recovery; and
- globally registered subscription evidence copied into a track.

### Excluded

- remote database and synchronization;
- recovery of facts never exported; and
- remote backup.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Migration advances after partial failure | Corrupted or unreadable index | Inject failure inside transaction and inspect version/data |
| Database becomes sole track evidence | Track unusable after index loss | Delete disposable index and scan portable folder |
| Binary evidence stored as SQLite blob | Large opaque database and poor portability | Inspect logical schema and track files |
| File succeeds but metadata commit fails | Orphaned file or false state | Inject commit failure and run recovery scan |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] Disposable current, supported-old, and unsupported-newer databases are prepared.
- [ ] A complete portable finalized track and an incomplete track fixture exist.
- [ ] All fixtures contain synthetic data and are independently backed up.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Current workspace | Global profile, two tracks, step states, and evidence metadata |
| TD-02 | Supported old schema | Minimal database at the oldest supported migration version |
| TD-03 | Migration failure | TD-02 copy with a controlled failure injected mid-migration |
| TD-04 | Unsupported newer schema | Version greater than the application's supported schema |
| TD-05 | Portable recovery set | One valid finalized track and one incomplete legacy track |
| TD-06 | Global evidence | Synthetic subscription PDF with a coverage period |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-PER-002` | Create and reopen TD-01. | Global values, tracks, workflow states, and evidence metadata round-trip exactly through native typed operations. | Not run | NOT RUN | — |
| 2 | `REQ-PER-002` | Inspect frontend/native command contracts and database contents. | No frontend raw-SQL command exists; evidence binaries remain files rather than SQLite blobs. | Static review found only narrow typed commands; SQLite stores evidence metadata and portable paths, with binary evidence copied as files. | PASS | [Central execution report](../../dev/acceptance-report.md); native invoke/schema review |
| 3 | `REQ-PER-003` | Open TD-02. | Ordered migrations complete transactionally, preserve data, and record the current schema version only after success. | Not run | NOT RUN | — |
| 4 | `REQ-PER-003` | Open TD-03. | Migration rolls back, prior data/version remain recoverable, and a controlled error is returned. | Not run | NOT RUN | — |
| 5 | `REQ-PER-003` | Open TD-04. | The application refuses to guess or downgrade and reports an unsupported-newer-schema error without mutation. | Schema version 99 was rejected; its version and sentinel table remained unchanged. | PASS | Rust `sqlite_migration_refuses_newer_schema_without_modifying_it`; [final suite](../../../.report/test-report-20260813-144834-suite-all-ok.md) |
| 6 | `REQ-ARC-005` | Inject a metadata-commit failure after a disposable evidence file is safely placed. | The app reports partial recoverable state without panic or source deletion; scan discovers the unindexed file. | Not run | NOT RUN | — |
| 7 | `REQ-PER-004` | Remove only the disposable SQLite index for TD-05 and start recovery. | Scan proposes both tracks from portable content and never modifies their files. | Not run | NOT RUN | — |
| 8 | `REQ-ARC-004` | Confirm reindex of TD-05. | Valid finalized snapshot is recovered after full verification; incomplete track retains exact missing/`NOT VERIFIED` facts. | Not run | NOT RUN | — |
| 9 | `REQ-PER-004` | Inspect unrecoverable mutable values. | UI/global facts that were never exported remain unset; no default is presented as historical truth. | Not run | NOT RUN | — |
| 10 | `REQ-PER-002` | Register TD-06 globally and select it for a track. | The global source remains registered; finalization preparation copies a contained track instance with role, relative path, size, and hash. | A signature-valid covering PDF remained globally registered while a contained track copy retained role, provenance ID, dates, bytes, size/path metadata, and SHA-256; the commercial end-to-end track finalized with it. | PASS | Rust global-subscription and end-to-end tests |

## Automated checks

```sh
cd src-tauri
cargo test sqlite_migrations_are_idempotent
cargo test workspace_creation_initializes_local_database
cargo test global_evidence_is_copied_portably
cargo test legacy_scan_is_read_only_and_not_verified
```

Expected Rust evidence is `tests::sqlite_migrations_are_idempotent`, `tests::workspace_creation_initializes_local_database`, `tests::global_evidence_is_copied_portably`, and `tests::legacy_scan_is_read_only_and_not_verified`. Attach migration, rollback, partial-failure, recovery-tree, and round-trip outputs when executed.

## Verification

The reviewer checks schema versions before/after, transaction rollback, controlled errors without panic, portable recovery results, honest unknowns, and global-evidence copy semantics.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Full state round-trip, data-preserving and failed migrations, commit failure, index-loss recovery, and unknown-value review remain unexecuted. | High | Product team | Execute steps 1, 3, 4, and 6–9 with retained database/tree evidence. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 2, 5, and 10 passed; seven mandatory steps remain `NOT RUN`.
- Residual risks: Crash reconciliation and index-independent recovery are not accepted yet; facts absent from portable files remain unrecoverable by design.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Local persistence and recovery](../../def/persistence.md)
- [Legacy track import](../../dev/legacy-track-import.md)
- [ATP-0001: Workspace creation](ATP-0001-workspace-creation-and-loading.md)
