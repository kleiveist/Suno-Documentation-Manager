<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Acceptance execution report — 2026-08-14

| Field | Value |
| --- | --- |
| Product | Suno Documentation Manager |
| Product version | `0.1.0` |
| Execution | 2026-08-13 through 2026-08-14 |
| Source identification | Regression implementation commit `b7e9797b277f0bcac58d4503049002e354cb93fb` (`🐛 Fix modal interaction and subscription evidence imports`); retained DEB/RPM packages still identify stabilization commit `af7d4846ffc329943fd33fed6d31e0cc372de571` |
| Automation host | Linux `7.1.4-arch1-1`, `x86_64`; Python 3.14.6; Node 26.4.0/npm 12.0.1; rustc/cargo 1.97.1; Tauri CLI 2.11.4; WebKitGTK 4.1/2.52.5 |
| Packaged-GUI environment | Disposable Debian 13.2 VM, KDE Plasma/Wayland, 2 vCPU, 4 GiB RAM; virtual NIC down during offline launch |
| Overall ATP result | **PARTIAL — 68 of 136 steps PASS; 68 steps NOT RUN; 0 FAIL** |
| Acceptance decision | **Not approved as fully accepted**; all 13 ATPs remain active |

## Outcome

The expanded automated core is green at implementation commit `b7e9797b…`.
Native tests cover the complete synthetic documentation path, all eight managed
golden documents, deterministic output,
adoption/collision preservation, artwork disclosure, exact SHA-256 sets,
certificate/data cross-checks, six certificate-publication failpoints,
database-commit rollback/recovery, migrations, revisions, workflow upgrade,
legacy reconciliation, containment, typed failure behavior, exact monthly/annual
subscription intervals, restart persistence, and no-clobber publication.
Frontend tests cover the workflow branches, progress/gate behavior, navigation, command
contracts, upgrade presentation, workspace-scoped UI reset, delegated modal
click routing, billing-cycle previews, and native command mapping.

The default host run passed 83 Rust tests with one removable-filesystem test
intentionally ignored unless an explicit disposable root is supplied. That
opt-in test was then run separately on the identified writable Samsung USB
volume (`SOURCE=/dev/sde1`, `FSTYPE=exfat`) and passed create-only publication,
copy/digest equality, source preservation, occupied-destination preservation,
and temporary-fixture cleanup without `EPERM`. This is evidence for that exact
Linux/exFAT environment, not a universal removable-filesystem claim.

The retained DEB and RPM packages were built earlier in the disposable Debian
VM from stabilization commit `af7d4846…`, before the modal/subscription
regression fix. The DEB was installed and the installed binary was launched with the VM network interface
down; failed name resolution independently confirmed the offline condition.
Native workspace and evidence dialogs, cancellation, workspace isolation, and
an offline application restart were observed and screenshots were retained.

This still does **not** complete the desktop acceptance protocol or accept the
regression fix in a packaged build. The packaged GUI track was not taken from
clean creation through all ten steps, independent
hash verification, finalization, copied-folder inspection, and a second offline
reopen. Real title/date/toggle retention inside the fixed modal and the complete
cadence-guided subscription picker remain packaged-GUI steps. The preliminary
GUI run also exposed an operator-entered malformed date that remained only in
the unsaved frontend draft and correctly blocked later
evidence import. No incomplete compound ATP step is promoted on the strength of
that partial run. No ATP is moved to `completed/`.

## Reproducible commands and evidence

The following checks were actually executed. Host toolchain commands were run
through `flatpak-spawn --host` where the Codex Flatpak did not contain Rust/npm:

```sh
python tools/control.py doctor
python tools/control.py tauri doctor
python tools/control.py test --suite all --report
cd src-tauri
cargo test --locked
SUNO_DOC_REMOVABLE_FS_TEST_ROOT=/path/to/disposable/exfat-root \
  cargo test --locked no_clobber_publish_works_on_configured_removable_filesystem -- --ignored --nocapture
cd ../frontend
npm test
npm run build
npm audit --omit=dev
cd ..
python tools/control.py build web
python tools/control.py release check
```

Before the opt-in command, `findmnt` with columns `TARGET,SOURCE,FSTYPE,OPTIONS`
identified the actual fixture as `/dev/sde1`,
`exfat`, and read/write. The private workspace path is intentionally not retained
in this report; the test itself creates and removes a uniquely named child.

The exact desktop build was executed inside the Debian VM:

```sh
python3 tools/control.py build desktop --target linux --bundles deb,rpm
```

| Check | Actual result | Evidence |
| --- | --- | --- |
| Full template suite at `b7e9797b…` | `OK`; tools 177 passed/21 skipped; schema, frontend, and Tauri passed; intentionally disabled suites skipped | [Full-suite report](../../.report/test-report-20260814-123936-suite-all-ok.md), SHA-256 `4ae1c789560a4fc3e50bfd0ae37b094d724792a5b50a5995f8ff53a26b981414` |
| Frontend at `b7e9797b…` | 4 files, 27 tests passed; TypeScript/Vite build passed | Current full-suite report and direct host build |
| Rust/Tauri at `b7e9797b…` | 83 passed, 0 failed, 1 opt-in filesystem test ignored by default | Direct `cargo test --locked` host run |
| Linux/exFAT opt-in at `b7e9797b…` | 1 passed, 0 failed on `/dev/sde1` (`exfat`, `rw`); isolated fixture removed | Rust `no_clobber_publish_works_on_configured_removable_filesystem`; direct host output |
| Production dependency audit | 0 vulnerabilities in the retained stabilization run | Direct `npm audit --omit=dev` output from stabilization execution |
| General doctor | Overall `OK`; backend intentionally disabled | Doctor output |
| Tauri doctor | Required desktop dependencies present; optional Corepack warning only | Tauri doctor output |
| Web package | Retained build passed before regression commit | [Web ZIP](../../.dist/web/suno-documentation-manager-web.zip) |
| Linux desktop packages | Retained DEB/RPM build passed inside Debian 13.2 at stabilization commit `af7d4846…`; not rebuilt from `b7e9797b…` | [Retained package directory](../../.report/packages/) |
| Release check | **Did not pass**: correctly rejected the dirty worktree; also warned that packages are unsigned | Release-check output; this is an open release/provenance deviation, not a product-test PASS |
| Packaged offline launch | Installed DEB launched with guest NIC down; app showed local/offline state | [Offline welcome](../../.report/acceptance-app-welcome-offline.png), [offline desktop](../../.report/acceptance-offline-desktop.png) |
| Native dialogs/restart | Native workspace/evidence pickers and cancellation observed; installed app restarted offline | [Workspace picker](../../.report/acceptance-native-workspace-picker.png), [cancel](../../.report/acceptance-picker-cancelled.png), [restart picker](../../.report/acceptance-restart-native-workspace-picker.png) |
| Workspace isolation | Workspace A initialized its own `.suno-doc/workspace.sqlite`; previously selected root data did not appear in A | [Workspace A](../../.report/acceptance-workspace-page-a.png), SQLite inspection through QEMU Guest Agent |

Retained pre-regression artifact digests:

```text
087cf97b51b1e57004176911194f8e755873de265779c010590e29a3b06d4df8  .dist/web/suno-documentation-manager-web.zip
8727557121c7ada5c88b143b765406375d627e7c7f1ec639852cb64ccd1812e2  .report/packages/Suno Documentation Manager_0.1.0_amd64.deb
ebb83d3237ecfa3b6fbadf91c33cb13048087f960149ba661152dad2f1d0b41c  .report/packages/Suno Documentation Manager-0.1.0-1.x86_64.rpm
```

The regression implementation and automated fixtures are identified by commit
`b7e9797b277f0bcac58d4503049002e354cb93fb`. The ATP/report synchronization is
the following documentation-only change. Rebuilding packages from a clean
checkout of that implementation (or its documentation-only descendant), then
repeating the package checks and GUI steps, remains the required provenance
proof. The earlier Flatpak-local report attempt that lacked `npm` and `cargo`
is not cited as product evidence.

## ATP execution matrix

`PASS` means the complete wording of the listed step is supported by an
executed automated check or retained review evidence. A step stays `NOT RUN`
when only part of a compound expectation was covered.

| ATP | PASS steps | NOT RUN steps | Result and precise remaining gap |
| --- | --- | --- | --- |
| [ATP-0001](../atp/active/ATP-0001-workspace-creation-and-loading.md) | 1, 3, 6 | 2, 4, 5, 7, 8, 9 | PARTIAL — complete create-cancel boundary, sibling-tree proof, settings round-trip, invalid target, and A→B→A switch remain |
| [ATP-0002](../atp/active/ATP-0002-track-creation.md) | 1, 2, 5, 6, 10, 11 | 3, 4, 7, 8, 9, 12 | PARTIAL — delegated modal routing passes; complete GUI field retention/dismissal, placeholder-tree inspection, snapshot assertion, editing/N/A branches, and collision sentinel remain |
| [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) | 2, 3 | 1, 4–10 | PARTIAL — ambiguity, complete integrity/index-loss recovery, adoption, cancellation, and injected failure remain |
| [ATP-0004](../atp/active/ATP-0004-document-generation.md) | 1, 4, 7, 8, 9, 10 | 2, 3, 5, 6 | PARTIAL — negative-only prose, snapshot stability, and stale-input propagation fixtures remain |
| [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) | 2, 3, 4, 5, 8 | 1, 6, 7, 9, 10 | PARTIAL — complete native picker step, stage combinations, positive content declarations, and removal remain |
| [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md) | 3, 5 | 1, 2, 4, 6–10 | PARTIAL — complete offline GUI action, visual clipping review, policy branches, and process-document review remain |
| [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md) | 1, 2, 4, 5, 9 | 3, 6, 7, 8, 10 | PARTIAL — full exclusion matrix, independent CLI check, compound mismatch steps, and copied-root verification remain |
| [ATP-0008](../atp/active/ATP-0008-finalization-gate.md) | 1, 7, 8, 10 | 2–6, 9 | PARTIAL — exact blocker/status/N/A matrices and post-readiness native race remain |
| [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) | 1, 2, 4, 6, 8, 10 | 3, 5, 7, 9 | PARTIAL — retained field-by-field artifact review and independent verification remain |
| [ATP-0010](../atp/active/ATP-0010-certificate-invalidation-and-revision.md) | 2, 3, 4, 5, 6, 7, 9, 10 | 1, 8 | PARTIAL — untouched packaged reopen and complete refinalization remain |
| [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) | 2, 3, 4, 5, 10, 11 | 1, 6, 7, 8, 9 | PARTIAL — exact one-invoice cadence persistence passes; full general-state round-trip, metadata failure, index-loss reconstruction, and honest unknown-value review remain |
| [ATP-0012](../atp/active/ATP-0012-filesystem-containment.md) | 1, 3, 4, 5, 6, 9, 10, 12 | 2, 7, 8, 11 | PARTIAL — identified Linux/exFAT publication passes; all-command path matrix, document-specific write/rename injection, full user-readable error matrix, and other filesystem claims remain |
| [ATP-0013](../atp/active/ATP-0013-end-to-end-offline-workflow.md) | 2, 3, 5, 6, 7, 8, 9 | 1, 4, 10, 11, 12 | PARTIAL — packaged offline launch is only partial evidence; complete native-picker workflow, restart, portable-copy, and runtime-observation steps remain |

## Known acceptance deviations

| ID | Scope | Observation | Follow-up required before full acceptance |
| --- | --- | --- | --- |
| DEV-ATP-001 | Release provenance | Commit `b7e9797b…` identifies the regression implementation and green source suites, but the retained packages identify older commit `af7d4846…`; the recorded release check also rejected its then-dirty worktree and packages are unsigned. | Commit this ATP/report synchronization, rebuild packages from a clean checkout, rerun suites/release check, and record the resulting package hashes. |
| DEV-ATP-002 | ATP-0001/0002/0005/0011/0013 | The older installed DEB launched offline and native dialogs were observed, but it does not contain the modal/cadence regression fix and the complete clean GUI workflow did not reach FINALIZED. | Build the current implementation and retain one uninterrupted native path including modal field retention, cadence-guided single-file registration, all ten steps, hash verification, finalization, restart, and portable copy. |
| DEV-ATP-003 | ATP-0003/0011 | Legacy ambiguity has fixtures, but full index-loss reconstruction of incomplete and valid finalized portable folders is not accepted. | Execute before/after tree digests and honest-unknown review for both fixture classes. |
| DEV-ATP-004 | ATP-0007/0009 | Native hash/parser/cross-check coverage is strong, but independent `sha256sum` verification of the final and copied portable folder was not completed. | Run both hash lists from the original and moved roots and retain command output. |
| DEV-ATP-005 | ATP-0012 | Static symlink rejection and the identified Linux/exFAT no-clobber fixture pass. Same-user concurrent path swaps remain outside the V0.1 threat model, and no result is generalized to other removable filesystems. | Keep the boundary operationally explicit; complete the remaining all-command/write/rename failure matrices and execute equivalent fixtures before claiming another filesystem. |

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | **PARTIAL — 68/136 PASS; 68 NOT RUN; 0 FAIL** | 2026-08-14 |
| Product acceptance owner | — | PENDING | — |
