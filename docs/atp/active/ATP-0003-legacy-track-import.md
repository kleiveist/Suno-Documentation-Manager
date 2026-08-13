<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0003: Legacy track import

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-14 |
| Executed | 2026-08-13/14 — partial automated legacy fixture execution |
| Requirement | [`REQ-LEG-001` through `REQ-LEG-003`](../../dev/legacy-track-import.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571`; package digests in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; disposable partial legacy track fixtures |

## Purpose

This plan verifies that existing track folders can be discovered and explicitly adopted without overwriting history or inventing unavailable facts.

## Objective

Accept legacy import when scan is read-only, known structures are mapped conservatively, unresolved facts block as `NOT VERIFIED`, and managed-document adoption preserves a verified archive copy.

## Scope

### Included

- candidate and known-structure discovery;
- known evidence, document, hash, manifest, and certificate recognition;
- conflicts and unknown-file reporting;
- `INCOMPLETE`/`NOT VERIFIED` presentation; and
- explicit document adoption with backup.

### Excluded

- automatic media repair;
- remote import; and
- inference of rights or authorship.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Scan mutates historical evidence | Irrecoverable data loss | Hash the entire fixture tree before and after scan |
| Filename is treated as proof | Fabricated facts | Use misleading and ambiguous names |
| Invalid old hash is trusted | False finalized presentation | Provide one mismatched protected file |
| Adoption destroys unmanaged content | Documentation loss | Force backup and injected write failure tests |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] Each fixture is copied into a new temporary workspace.
- [ ] Pre-scan tree paths, sizes, and SHA-256 values are recorded.
- [ ] Fixtures contain synthetic data only.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Partial legacy track | Known `01_RELEASE/`, `02_SUNO/`, and `03_DOCUMENTATION/` directories with one real disposable file |
| TD-02 | Ambiguous track | Duplicate candidate artwork roles and unknown sibling files |
| TD-03 | Unmanaged document | Existing `03_DOCUMENTATION/README.md` with a sentinel and no managed-template marker |
| TD-04 | Integrity fixtures | One valid and one mismatched `SHA256SUMS.txt`; one malformed manifest |
| TD-05 | Valid finalized portable fixture | Supported manifest, workflow version, certificate, and matching hashes |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-LEG-001` | Scan a workspace containing TD-01 through TD-05. | Each candidate is listed with found, missing, conflict, unknown, and warning information. | Not run | NOT RUN | — |
| 2 | `REQ-LEG-001` | Compare complete pre-scan and post-scan tree digests before confirming import. | Every path and byte remains unchanged; no management write occurs inside a candidate. | Full relative-path/byte snapshots were identical before scan and after initial and repeat scans. | PASS | Rust `legacy_scan_is_read_only_and_indexes_evidence_as_historically_unverified` |
| 3 | `REQ-LEG-002` | Review TD-01's missing production facts. | Unknown values remain absent and mandatory unresolved steps show `NOT VERIFIED` or incomplete. | The partial candidate retained absent facts/default snapshot and all discovered evidence was indexed as historically unverified; profile adoption required an explicit action. | PASS | Rust legacy scan/adoption assertions; native model review |
| 4 | `REQ-LEG-001` | Review TD-02. | Duplicate roles and unknown files are reported for user selection; neither is silently classified or removed. | Not run | NOT RUN | — |
| 5 | `REQ-LEG-002` | Verify TD-04. | The mismatch and malformed manifest are rejected and cannot produce a valid finalized state. | Not run | NOT RUN | — |
| 6 | `REQ-LEG-001` | Verify TD-05. | The supported contained manifest and all hashes validate; the finalized presentation is recovered without rewriting artifacts. | Not run | NOT RUN | — |
| 7 | `REQ-LEG-003` | Request adoption of TD-03. | The UI previews existing and proposed content and requires explicit confirmation. | Not run | NOT RUN | — |
| 8 | `REQ-LEG-003` | Cancel adoption. | TD-03 and the tree remain byte-identical. | Not run | NOT RUN | — |
| 9 | `REQ-LEG-003` | Confirm adoption. | A verified unique backup is created below `.archive/`, then the managed document is atomically written and indexed. | Not run | NOT RUN | — |
| 10 | `REQ-LEG-003` | Inject a backup or destination-write failure in a fresh copy. | Original content remains intact, no successful adoption is recorded, and a controlled error is returned. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test legacy_scan_is_read_only_and_not_verified
cargo test document_generation_is_deterministic_and_requires_adoption
```

Expected Rust evidence is `tests::legacy_scan_is_read_only_and_not_verified` and `tests::document_generation_is_deterministic_and_requires_adoption`. Attach fixture-tree comparisons and test output when execution starts.

## Verification

The reviewer confirms zero scan-time mutations, honest unknowns, explicit mappings, invalid-integrity rejection, and verified backup-before-write ordering.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Ambiguous/conflicting candidates, malformed/valid finalized integrity, managed-document adoption/cancel, and failure injection remain unexecuted. | High | Product team | Execute steps 1 and 4–10 with retained fixture artifacts. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 2 and 3 passed for a partial legacy candidate; eight mandatory steps remain `NOT RUN`.
- Residual risks: Ambiguity handling, finalized recovery, and adoption rollback have no executed fixture evidence.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Legacy track import design](../../dev/legacy-track-import.md)
- [Persistence and recovery](../../def/persistence.md)
- [ATP-0012: Filesystem containment](ATP-0012-filesystem-containment.md)
