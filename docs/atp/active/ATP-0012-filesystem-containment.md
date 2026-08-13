<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0012: Filesystem containment and safe writes

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-13 |
| Executed | 2026-08-13 — partial automated execution |
| Requirement | [`REQ-ARC-002`, `REQ-ARC-003`](../../def/product-architecture.md#product-requirements-and-atp-mapping), [`REQ-LEG-004`](../../dev/legacy-track-import.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; unversioned source tree; Linux package digests in the central report |
| Environment | Linux `7.0.8-1-cachyos` `x86_64`; temporary workspaces with Unix symbolic-link support |

## Purpose

This plan verifies canonical root containment, traversal and absolute-path rejection, symbolic-link escape handling, collision protection, atomic writes, narrow commands, and controlled filesystem errors.

## Objective

Accept the native filesystem boundary when every read/write target is derived from a named operation and contained root, hostile paths cannot escape, existing files are not silently replaced, and expected failures leave no partial managed claim.

## Scope

### Included

- workspace and track canonicalization;
- traversal, absolute path, and separator injection;
- escaping symbolic links;
- contained symlinks according to the product policy;
- collision and source-preservation rules;
- atomic generated-document writes; and
- least-privilege Tauri command/capability review.

### Excluded

- operating-system compromise;
- malicious or concurrent modification by another process running as the same operating-system user with direct workspace write access; and
- remote path or network-share guarantees not explicitly supported.

## Version 0.1 threat-model note

The product rejects absolute inputs, traversal, and symbolic-link components that exist when native path validation runs. Its version 0.1 implementation is path-based rather than descriptor-relative across the complete operation. It therefore does not claim protection when another process with the same user's workspace permissions swaps a checked component before the later open, copy, rename, or delete.

No symbolic-link path is intentionally supported for product-managed operations, including a link whose current target is contained. Existing-link rejection does not close the concurrent-swap race. Race-resistant descriptor-relative operations and a target-platform adversarial fixture remain ATP work, so the version 0.1 requirement must not be reported as fully satisfied on the basis of static-link tests alone.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Canonicalization occurs after write | Outside file modification | Place sentinels and compare before/after every hostile input |
| Symlink escapes root | Access outside authorized workspace | Test existing link and swapped-link scenarios where feasible |
| Same-user link swap races path validation | Outside read or write despite an earlier contained result | Keep shared hostile writers outside the version 0.1 threat model; retain descriptor-relative hardening and adversarial race execution as open work |
| Failed rename leaves partial content | Corrupt managed document | Inject write/rename failure and inspect old/temp files |
| Generic native command broadens authority | Webview compromise gains filesystem control | Inspect registered commands and Tauri capabilities |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment, operating system, filesystem, and build are identified.
- [ ] Disposable workspace and outside-sentinel directories are prepared.
- [ ] Symbolic-link capability is available or a justified platform N/A is approved.
- [ ] No production or private files are in or near the fixture.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Traversal inputs | `../outside`, nested `a/../../outside`, encoded/separator variants accepted by the command input type |
| TD-02 | Absolute inputs | Platform-native absolute file and directory strings pointing to an outside sentinel |
| TD-03 | Escaping symbolic link | Link inside workspace targeting TD-02's outside directory |
| TD-04 | Contained target and collision | Valid nested track destination with an existing sentinel file |
| TD-05 | Atomic-write fixture | Existing managed document plus injected temporary-write and rename failures |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-ARC-003` | Create/open a normal contained workspace and track. | Canonical roots are accepted and named operations reach only calculated contained destinations. | Native temporary workspace and track creation succeeded through calculated contained roots. | PASS | Rust workspace, track, and end-to-end tests; [final suite](../../../.report/test-report-20260813-144834-suite-all-ok.md) |
| 2 | `REQ-ARC-003` | Submit each TD-01 value through every relevant typed path-bearing use case. | Each escape is rejected before I/O; outside sentinels and workspace state remain unchanged. | Not run | NOT RUN | — |
| 3 | `REQ-ARC-003` | Submit TD-02. | Absolute injected paths are rejected; portable records never store them. | The contained-path test rejected a native absolute path; integrated evidence and manifest assertions contained only track-relative portable paths and no workspace root. | PASS | Rust `safe_path_rejects_traversal_and_absolute_paths` and end-to-end manifest assertions |
| 4 | `REQ-LEG-004` | Scan/import/write through TD-03. | An escaping symbolic link is rejected without reading or writing its outside target. | Unix tests rejected an escaping path component and a symlinked `.suno-doc`; no outside database was created. | PASS | Rust `safe_path_rejects_symlink_escape` and `persistence_rejects_symlinked_admin_directory` |
| 5 | `REQ-ARC-003` | Exercise an explicitly supported contained symlink case. | Behavior matches the documented policy and cannot be swapped into an escape between validation and write. | Not run | NOT RUN | — |
| 6 | `REQ-ARC-003` | Import a disposable source into occupied TD-04. | Collision is reported; source and destination remain byte-identical and no success metadata is committed. | Duplicate evidence import returned `Collision`; both source and managed destination bytes remained unchanged. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 7 | `REQ-ARC-003` | Generate over TD-05 with injected temporary-write failure. | Previous document remains intact, no partial destination is exposed, and a controlled error is returned. | Not run | NOT RUN | — |
| 8 | `REQ-ARC-003` | Inject rename failure in a fresh TD-05 copy. | Previous document remains valid, temporary state is recoverable/reported, and index freshness is not falsely advanced. | Not run | NOT RUN | — |
| 9 | `REQ-ARC-002` | Inspect registered Tauri commands and frontend calls. | Only narrow named product operations exist; no arbitrary SQL, path/action, shell, or unrestricted filesystem command is exposed. | Static review found only named product commands; searches found no generic SQL/file/shell command surface. | PASS | [Central execution report](../../dev/acceptance-report.md); `src-tauri/src/main.rs` |
| 10 | `REQ-ARC-002` | Inspect Tauri capability configuration. | No global filesystem allowlist such as `/**` exists; permissions are the minimum required for named native operations. | The only declared window permission is `core:default`; no filesystem glob/allowlist is present. | PASS | `src-tauri/capabilities/default.json`; Tauri structure check |
| 11 | `REQ-ARC-003` | Trigger expected permission, missing-file, malformed-path, and read errors. | Each returns a stable user-readable error without Rust panic or application termination. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test safe_path_rejects_traversal
cargo test safe_path_rejects_symlink_escape
cargo test atomic_write_replaces_complete_file
cargo test evidence_import_validates_type_and_rejects_collision
```

Static review commands from the repository root:

```sh
rg -n "execute_sql|execute_file_operation|allow.*\/\*\*|Command::new" frontend src-tauri
```

Expected Rust evidence is `tests::safe_path_rejects_traversal`, Unix-only `tests::safe_path_rejects_symlink_escape`, `tests::atomic_write_replaces_complete_file`, and `tests::evidence_import_validates_type_and_rejects_collision`. Expected frontend error-boundary evidence is `toUserMessage > presents structured/string/unknown errors` and `isTauriRuntime > distinguishes browser demo` in `frontend/src/api/desktop.test.ts`.

## Verification

Attach platform details, before/after sentinel hashes, path-case matrix, symbolic-link result or justified N/A reason, failure-injection output, capability review, and automated report.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | The all-command traversal matrix, same-user symbolic-link swap race, write/rename failure injection, and complete filesystem error matrix remain unresolved or unexecuted. | High | Product team | Introduce descriptor-relative race hardening or explicitly retain the narrower threat model, then execute steps 2, 5, 7, 8, and 11 with outside sentinels, a concurrent swap fixture, and injected failures. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 3, 4, 6, 9, and 10 passed; five mandatory steps remain `NOT RUN`.
- Residual risks: Symbolic-link swap races and atomic-write failure behavior still require target-specific execution.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Product architecture](../../def/product-architecture.md#workspace-and-path-boundary)
- [Legacy track import](../../dev/legacy-track-import.md)
- [Local persistence and recovery](../../def/persistence.md)
