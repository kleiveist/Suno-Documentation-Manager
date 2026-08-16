<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0012: Filesystem containment and safe writes

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-14 |
| Executed | 2026-08-13/14 — partial automated execution |
| Requirement | [`REQ-ARC-002`, `REQ-ARC-003`, `REQ-ARC-006`](../../def/product-architecture.md#product-requirements-and-atp-mapping), [`REQ-LEG-004`](../../dev/legacy-track-import.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; regression implementation commit `b7e9797b277f0bcac58d4503049002e354cb93fb` (`🐛 Fix modal interaction and subscription evidence imports`); package rebuild remains open in the central report |
| Environment | Linux `7.1.4-arch1-1` `x86_64`; temporary Unix/symbolic-link workspaces plus an identified writable Samsung USB `/dev/sde1` mount with `FSTYPE=exfat` |

## Purpose

This plan verifies canonical root containment, traversal and absolute-path rejection, symbolic-link escape handling, collision protection, atomic writes, narrow commands, controlled filesystem errors, and filesystem-scoped no-clobber publication.

## Objective

Accept the native filesystem boundary when every read/write target is derived from a named operation and contained root, hostile paths cannot escape, existing files are not silently replaced, expected failures leave no partial managed claim, and publication works on each explicitly accepted local filesystem.

## Scope

### Included

- workspace and track canonicalization;
- traversal, absolute path, and separator injection;
- escaping symbolic links;
- contained symlinks according to the product policy;
- collision and source-preservation rules;
- atomic generated-document writes;
- create-only/evidence-copy publication on an identified Linux/exFAT fixture; and
- least-privilege Tauri command/capability review.

### Excluded

- operating-system compromise;
- malicious or concurrent modification by another process running as the same operating-system user with direct workspace write access;
- removable filesystems and operating systems other than the explicitly identified fixture; and
- remote path or network-share guarantees not explicitly supported.

## Version 0.1 threat-model note

The product rejects absolute inputs, traversal, and symbolic-link components that exist when native path validation runs. Its version 0.1 implementation is path-based rather than descriptor-relative across the complete operation. It therefore does not claim protection when another process with the same user's workspace permissions swaps a checked component before the later open, copy, rename, or delete.

No symbolic-link path is intentionally supported for product-managed operations, including a link whose current target is contained. Existing-link rejection does not close the concurrent-swap race. Version 0.1 explicitly accepts that residual risk for workspaces without an untrusted concurrent same-user writer; it must never be described as descriptor-relative or fully race-safe.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Canonicalization occurs after write | Outside file modification | Place sentinels and compare before/after every hostile input |
| Symlink escapes root | Access outside authorized workspace | Test existing link and swapped-link scenarios where feasible |
| Same-user link swap races path validation | Outside read or write despite an earlier contained result | Accepted version 0.1 residual risk: do not use a workspace shared with an untrusted same-user writer; retain descriptor-relative hardening as a post-0.1 improvement |
| Failed rename leaves partial content | Corrupt managed document | Inject write/rename failure and inspect old/temp files |
| Publication depends on an unsupported filesystem primitive | A removable-drive import fails with `Operation not permitted` | Execute create, copy, digest, collision, source-preservation, and cleanup assertions directly on every filesystem claimed as supported |
| Generic native command broadens authority | Webview compromise gains filesystem control | Inspect registered commands and Tauri capabilities |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment, operating system, filesystem, and build are identified.
- [ ] Disposable workspace and outside-sentinel directories are prepared.
- [ ] Symbolic-link capability is available or a justified platform N/A is approved.
- [ ] Every test write is confined to a newly created disposable child; no existing production or private file is selected as test data.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | Traversal inputs | `../outside`, nested `a/../../outside`, encoded/separator variants accepted by the command input type |
| TD-02 | Absolute inputs | Platform-native absolute file and directory strings pointing to an outside sentinel |
| TD-03 | Escaping symbolic link | Link inside workspace targeting TD-02's outside directory |
| TD-04 | Contained target and collision | Valid nested track destination with an existing sentinel file |
| TD-05 | Atomic-write fixture | Existing managed document plus injected temporary-write and rename failures |
| TD-06 | Linux/exFAT publication fixture | Isolated disposable directory created by the opt-in, ignored-by-default Rust test inside the identified writable Samsung USB `/dev/sde1` exFAT mount; no production file is selected or modified |
| TD-07 | Final release naming | Valid WAV and MP3 sources, readable and filesystem-invalid track-title characters, and an occupied title-based target |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-ARC-003` | Create/open a normal contained workspace and track. | Canonical roots are accepted and named operations reach only calculated contained destinations. | Native temporary workspace and track creation succeeded through calculated contained roots. | PASS | Rust workspace, track, and end-to-end tests; [final suite](../../../.report/test-report-20260813-232332-suite-all-ok.md) |
| 2 | `REQ-ARC-003` | Submit each TD-01 value through every relevant typed path-bearing use case. | Each escape is rejected before I/O; outside sentinels and workspace state remain unchanged. | Not run | NOT RUN | — |
| 3 | `REQ-ARC-003` | Submit TD-02. | Absolute injected paths are rejected; portable records never store them. | The contained-path test rejected a native absolute path; integrated evidence and manifest assertions contained only track-relative portable paths and no workspace root. | PASS | Rust `safe_path_rejects_traversal_and_absolute_paths` and end-to-end manifest assertions |
| 4 | `REQ-LEG-004` | Scan/import/write through TD-03. | An escaping symbolic link is rejected without reading or writing its outside target. | Unix tests rejected an escaping path component and a symlinked `.suno-doc`; no outside database was created. | PASS | Rust `safe_path_rejects_symlink_escape` and `persistence_rejects_symlinked_admin_directory` |
| 5 | `REQ-ARC-003` | Exercise a symlink whose current target is contained. | The operation rejects it according to the no-managed-symlinks policy; this static test does not claim protection from a later same-user component swap. | A Unix test created a link to a contained directory and native path validation rejected it as a symbolic-link component. | PASS | Rust `safe_path_rejects_symlink_escape`; accepted version 0.1 threat-model boundary above |
| 6 | `REQ-ARC-003` | Import a disposable source into occupied TD-04. | Collision is reported; source and destination remain byte-identical and no success metadata is committed. | Duplicate evidence import returned `Collision`; both source and managed destination bytes remained unchanged. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 6a | `REQ-ARC-003` | Import and rename TD-07 final audio through the native track operations. | The managed path uses the safe track title, preserves WAV/MP3 extension, updates evidence metadata, rejects unsafe path semantics, and never overwrites an occupied target. | Unit and application tests produced title-based WAV/MP3 paths, sanitized invalid filename characters, kept source/destination collision bytes unchanged, and updated the relative evidence path after a native title rename. A rename collision rolled the folder, file, and metadata back. | PASS | Rust `release_import_uses_a_safe_track_title_and_preserves_the_actual_extension`, `release_import_never_overwrites_an_existing_track_title_target`, `changing_a_track_title_renames_its_managed_folder`, `track_title_release_collision_rolls_back_folder_file_and_metadata` |
| 7 | `REQ-ARC-003` | Generate over TD-05 with injected temporary-write failure. | Previous document remains intact, no partial destination is exposed, and a controlled error is returned. | Not run | NOT RUN | — |
| 8 | `REQ-ARC-003` | Inject rename failure in a fresh TD-05 copy. | Previous document remains valid, temporary state is recoverable/reported, and index freshness is not falsely advanced. | Not run | NOT RUN | — |
| 9 | `REQ-ARC-002` | Inspect registered Tauri commands and frontend calls. | Only narrow named product operations exist; no arbitrary SQL, path/action, shell, or unrestricted filesystem command is exposed. | Static review found only named product commands; searches found no generic SQL/file/shell command surface. | PASS | [Central execution report](../../dev/acceptance-report.md); `src-tauri/src/main.rs` |
| 10 | `REQ-ARC-002` | Inspect Tauri capability configuration. | No global filesystem allowlist such as `/**` exists; permissions are the minimum required for named native operations. | The only declared window permission is `core:default`; no filesystem glob/allowlist is present. | PASS | `src-tauri/capabilities/default.json`; Tauri structure check |
| 11 | `REQ-ARC-003` | Trigger expected permission, missing-file, malformed-path, and read errors. | Each returns a stable user-readable error without Rust panic or application termination. | Not run | NOT RUN | — |
| 12 | `REQ-ARC-006` | Confirm TD-06 with `findmnt`, then run the opt-in no-clobber fixture with `SUNO_DOC_REMOVABLE_FS_TEST_ROOT` naming its disposable parent. | Create-only and copy publication complete without `EPERM`; source and destination digests match after the first copy; changing the source and retrying reports collision without changing the occupied destination; the source remains present and no temporary fixture remains. | `findmnt` identified `/dev/sde1` as writable `exfat`. The opt-in Rust test passed all create/copy/digest/source/collision/destination/cleanup assertions in an isolated child directory, and automatic cleanup left no `.suno-doc-fs-compat-*` directory. | PASS | Rust `no_clobber_publish_works_on_configured_removable_filesystem`; exact command and environment in the central execution report |

## Automated checks

```sh
cd src-tauri
cargo test safe_path_rejects_traversal_and_absolute_paths
cargo test safe_path_rejects_symlink_escape
cargo test atomic_writes_publish_complete_bytes_and_never_clobber_new_files
cargo test no_clobber_publish_preserves_a_destination_created_after_staging
cargo test copy_new_publishes_complete_bytes_and_preserves_the_source
cargo test atomic_and_copy_failures_preserve_existing_state_and_clean_temporaries
cargo test evidence_import_validates_type_preserves_source_and_rejects_collision
```

Opt-in filesystem execution (the configured root must be disposable and writable):

```sh
findmnt -T /path/to/disposable/exfat-root -o TARGET,SOURCE,FSTYPE,OPTIONS
SUNO_DOC_REMOVABLE_FS_TEST_ROOT=/path/to/disposable/exfat-root \
  cargo test --locked no_clobber_publish_works_on_configured_removable_filesystem -- --ignored --nocapture
```

Static review commands from the repository root:

```sh
rg -n "execute_sql|execute_file_operation|allow.*\/\*\*|Command::new" frontend src-tauri
```

Expected Rust evidence is `safe_path_rejects_traversal_and_absolute_paths`, Unix-only `safe_path_rejects_symlink_escape`, `atomic_writes_publish_complete_bytes_and_never_clobber_new_files`, `no_clobber_publish_preserves_a_destination_created_after_staging`, `copy_new_publishes_complete_bytes_and_preserves_the_source`, `atomic_and_copy_failures_preserve_existing_state_and_clean_temporaries`, `evidence_import_validates_type_preserves_source_and_rejects_collision`, and the opt-in `no_clobber_publish_works_on_configured_removable_filesystem`. Expected frontend error-boundary evidence is `toUserMessage > presents structured/string/unknown errors` and `isTauriRuntime > distinguishes browser demo` in `frontend/src/api/desktop.test.ts`.

## Verification

Attach platform details, before/after sentinel hashes, path-case matrix, symbolic-link result or justified N/A reason, failure-injection output, capability review, and automated report.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | The all-command traversal matrix, write/rename failure injection, and complete filesystem error matrix remain unresolved or unexecuted. The same-user swap race is an accepted version 0.1 boundary, not a claimed protection. | High | Product team | Execute steps 2, 7, 8, and 11 with outside sentinels and injected failures; consider descriptor-relative hardening after version 0.1. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 1, 3–6, 9, 10, and 12 passed; four mandatory steps remain `NOT RUN`.
- Residual risks: Same-user symbolic-link swap races are explicitly accepted outside the version 0.1 threat model; document-specific write/rename failure behavior and filesystems beyond the identified Linux/exFAT fixture still require their own execution.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Product architecture](../../def/product-architecture.md#workspace-and-path-boundary)
- [Legacy track import](../../dev/legacy-track-import.md)
- [Local persistence and recovery](../../def/persistence.md)
