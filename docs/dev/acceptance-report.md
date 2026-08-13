<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Acceptance execution report — 2026-08-14

| Field | Value |
| --- | --- |
| Product | Suno Documentation Manager |
| Product version | `0.1.0` |
| Execution | 2026-08-13 through 2026-08-14 |
| Source identification | Stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571` (`🛡️ Harden workflow lifecycle and acceptance coverage`) |
| Automation host | Linux `7.1.4-arch1-1`, `x86_64`; Python 3.14.6; Node 26.4.0/npm 12.0.1; rustc/cargo 1.97.1; Tauri CLI 2.11.4; WebKitGTK 4.1/2.52.5 |
| Packaged-GUI environment | Disposable Debian 13.2 VM, KDE Plasma/Wayland, 2 vCPU, 4 GiB RAM; virtual NIC down during offline launch |
| Overall ATP result | **PARTIAL — 65 of 132 steps PASS; 67 steps NOT RUN; 0 FAIL** |
| Acceptance decision | **Not approved as fully accepted**; all 13 ATPs remain active |

## Outcome

The expanded automated core is green. Native tests cover the complete synthetic
documentation path, all eight managed golden documents, deterministic output,
adoption/collision preservation, artwork disclosure, exact SHA-256 sets,
certificate/data cross-checks, six certificate-publication failpoints,
database-commit rollback/recovery, migrations, revisions, workflow upgrade,
legacy reconciliation, containment, and typed failure behavior. Frontend tests
cover the workflow branches, progress/gate behavior, navigation, command
contracts, upgrade presentation, and workspace-scoped UI reset.

Exact DEB and RPM packages were built in the disposable Debian VM. The DEB was
installed and the installed binary was launched with the VM network interface
down; failed name resolution independently confirmed the offline condition.
Native workspace and evidence dialogs, cancellation, workspace isolation, and
an offline application restart were observed and screenshots were retained.

This still does **not** complete the desktop acceptance protocol. The packaged
GUI track was not taken from clean creation through all ten steps, independent
hash verification, finalization, copied-folder inspection, and a second offline
reopen. The preliminary GUI run also exposed an operator-entered malformed date
that remained only in the unsaved frontend draft and correctly blocked later
evidence import. No incomplete compound ATP step is promoted on the strength of
that partial run. No ATP is moved to `completed/`.

## Reproducible commands and evidence

The following checks were actually executed. Host toolchain commands were run
through `flatpak-spawn --host` where the Codex Flatpak did not contain Rust/npm:

```sh
python tools/control.py doctor
python tools/control.py tauri doctor
python tools/control.py test --suite all --report
cd src-tauri && cargo test --locked
cd ../frontend && npm test && npm run build && npm audit --omit=dev
cd ..
python tools/control.py build web
python tools/control.py release check
```

The exact desktop build was executed inside the Debian VM:

```sh
python3 tools/control.py build desktop --target linux --bundles deb,rpm
```

| Check | Actual result | Evidence |
| --- | --- | --- |
| Full template suite | `OK`; tools 177 passed/21 skipped; schema, frontend, and Tauri passed; intentionally disabled suites skipped | [Full-suite report](../../.report/test-report-20260813-232332-suite-all-ok.md), SHA-256 `bbb216ff7ac2f77022a8f5bf824df52aff835a00692f3222c1a893d6a026b93d` |
| Frontend | 3 files, 22 tests passed; TypeScript/Vite build passed | Full-suite report and direct host run |
| Rust/Tauri | 75 tests passed, 0 failed | Direct `cargo test --locked` host run |
| Production dependency audit | 0 vulnerabilities | Direct `npm audit --omit=dev` host run |
| General doctor | Overall `OK`; backend intentionally disabled | Doctor output |
| Tauri doctor | Required desktop dependencies present; optional Corepack warning only | Tauri doctor output |
| Web package | Build passed | [Web ZIP](../../.dist/web/suno-documentation-manager-web.zip) |
| Linux desktop packages | Exact DEB/RPM build passed inside Debian 13.2 | [Retained package directory](../../.report/packages/) |
| Release check | **Did not pass**: correctly rejected the dirty worktree; also warned that packages are unsigned | Release-check output; this is an open release/provenance deviation, not a product-test PASS |
| Packaged offline launch | Installed DEB launched with guest NIC down; app showed local/offline state | [Offline welcome](../../.report/acceptance-app-welcome-offline.png), [offline desktop](../../.report/acceptance-offline-desktop.png) |
| Native dialogs/restart | Native workspace/evidence pickers and cancellation observed; installed app restarted offline | [Workspace picker](../../.report/acceptance-native-workspace-picker.png), [cancel](../../.report/acceptance-picker-cancelled.png), [restart picker](../../.report/acceptance-restart-native-workspace-picker.png) |
| Workspace isolation | Workspace A initialized its own `.suno-doc/workspace.sqlite`; previously selected root data did not appear in A | [Workspace A](../../.report/acceptance-workspace-page-a.png), SQLite inspection through QEMU Guest Agent |

Artifact digests:

```text
087cf97b51b1e57004176911194f8e755873de265779c010590e29a3b06d4df8  .dist/web/suno-documentation-manager-web.zip
8727557121c7ada5c88b143b765406375d627e7c7f1ec639852cb64ccd1812e2  .report/packages/Suno Documentation Manager_0.1.0_amd64.deb
ebb83d3237ecfa3b6fbadf91c33cb13048087f960149ba661152dad2f1d0b41c  .report/packages/Suno Documentation Manager-0.1.0-1.x86_64.rpm
```

The implementation and fixtures under test are now identified by commit
`af7d4846ffc329943fd33fed6d31e0cc372de571`. The ATP/report synchronization in
this document is a later uncommitted documentation-only change. Rebuilding the
packages from a completely clean checkout of that commit and repeating the
package checks remains the preferred final provenance proof.

## ATP execution matrix

`PASS` means the complete wording of the listed step is supported by an
executed automated check or retained review evidence. A step stays `NOT RUN`
when only part of a compound expectation was covered.

| ATP | PASS steps | NOT RUN steps | Result and precise remaining gap |
| --- | --- | --- | --- |
| [ATP-0001](../atp/active/ATP-0001-workspace-creation-and-loading.md) | 1, 3, 6 | 2, 4, 5, 7, 8, 9 | PARTIAL — complete create-cancel boundary, sibling-tree proof, settings round-trip, invalid target, and A→B→A switch remain |
| [ATP-0002](../atp/active/ATP-0002-track-creation.md) | 1, 2, 5, 6, 10 | 3, 4, 7, 8, 9 | PARTIAL — placeholder-tree inspection, snapshot assertion, editing/N/A branches, and collision sentinel remain |
| [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) | 2, 3 | 1, 4–10 | PARTIAL — ambiguity, complete integrity/index-loss recovery, adoption, cancellation, and injected failure remain |
| [ATP-0004](../atp/active/ATP-0004-document-generation.md) | 1, 4, 7, 8, 9, 10 | 2, 3, 5, 6 | PARTIAL — negative-only prose, snapshot stability, and stale-input propagation fixtures remain |
| [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) | 2, 3, 4, 5, 8 | 1, 6, 7, 9, 10 | PARTIAL — complete native picker step, stage combinations, positive content declarations, and removal remain |
| [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md) | 3, 5 | 1, 2, 4, 6–10 | PARTIAL — complete offline GUI action, visual clipping review, policy branches, and process-document review remain |
| [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md) | 1, 2, 4, 5, 9 | 3, 6, 7, 8, 10 | PARTIAL — full exclusion matrix, independent CLI check, compound mismatch steps, and copied-root verification remain |
| [ATP-0008](../atp/active/ATP-0008-finalization-gate.md) | 1, 7, 8, 10 | 2–6, 9 | PARTIAL — exact blocker/status/N/A matrices and post-readiness native race remain |
| [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) | 1, 2, 4, 6, 8, 10 | 3, 5, 7, 9 | PARTIAL — retained field-by-field artifact review and independent verification remain |
| [ATP-0010](../atp/active/ATP-0010-certificate-invalidation-and-revision.md) | 2, 3, 4, 5, 6, 7, 9, 10 | 1, 8 | PARTIAL — untouched packaged reopen and complete refinalization remain |
| [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) | 2, 3, 4, 5, 10 | 1, 6, 7, 8, 9 | PARTIAL — full round-trip, metadata failure, index-loss reconstruction, and honest unknown-value review remain |
| [ATP-0012](../atp/active/ATP-0012-filesystem-containment.md) | 1, 3, 4, 5, 6, 9, 10 | 2, 7, 8, 11 | PARTIAL — all-command path matrix, document-specific write/rename injection, and full user-readable error matrix remain |
| [ATP-0013](../atp/active/ATP-0013-end-to-end-offline-workflow.md) | 2, 3, 5, 6, 7, 8, 9 | 1, 4, 10, 11, 12 | PARTIAL — packaged offline launch is only partial evidence; complete native-picker workflow, restart, portable-copy, and runtime-observation steps remain |

## Known acceptance deviations

| ID | Scope | Observation | Follow-up required before full acceptance |
| --- | --- | --- | --- |
| DEV-ATP-001 | Release provenance | Commit `af7d4846…` identifies the tested implementation, but the recorded release check ran before this documentation-only synchronization and rejected a dirty worktree; packages are unsigned. | Commit the ATP/report synchronization, rebuild packages from a clean checkout, rerun suites/release check, and record the resulting package hashes. |
| DEV-ATP-002 | ATP-0001/0005/0013 | The installed DEB launched offline and native dialogs were observed, but the complete clean GUI workflow did not reach FINALIZED. | Repeat from a clean workspace and retain one uninterrupted native path through all ten steps, hash verification, finalization, restart, and portable copy. |
| DEV-ATP-003 | ATP-0003/0011 | Legacy ambiguity has fixtures, but full index-loss reconstruction of incomplete and valid finalized portable folders is not accepted. | Execute before/after tree digests and honest-unknown review for both fixture classes. |
| DEV-ATP-004 | ATP-0007/0009 | Native hash/parser/cross-check coverage is strong, but independent `sha256sum` verification of the final and copied portable folder was not completed. | Run both hash lists from the original and moved roots and retain command output. |
| DEV-ATP-005 | ATP-0012 | Static symlink rejection is accepted for V0.1; same-user concurrent path swaps are explicitly outside the V0.1 threat model. | Document the boundary operationally; complete the remaining all-command and write/rename failure matrices. |

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | **PARTIAL — 65/132 PASS; 67 NOT RUN; 0 FAIL** | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |
