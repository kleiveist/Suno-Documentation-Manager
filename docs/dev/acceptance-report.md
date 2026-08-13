<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Acceptance execution report — 2026-08-13

| Field | Value |
| --- | --- |
| Product | Suno Documentation Manager |
| Product version | `0.1.0` |
| Execution date | 2026-08-13 |
| Source identification | Unversioned generated product directory; no Git commit is available |
| Host | Linux `7.0.8-1-cachyos`, `x86_64` |
| Overall ATP result | **PARTIAL — 48 of 132 steps PASS; 84 steps NOT RUN** |
| Acceptance decision | Not approved as fully accepted; all 13 ATPs remain active |

## Outcome

The automated core path is green. A native Rust integration test creates a
workspace and track, imports real disposable evidence copies, generates all
eight managed documents, calculates and verifies the exact SHA-256 set, passes
the native finalization gate, publishes the three certificate artifacts, and
verifies the certificate. Separate tests cover revision archival and recovery,
filesystem/database commit windows, read-only legacy reconciliation, global
subscription coverage/provenance, containment, symlink rejection, evidence
collisions, local artwork disclosure, migrations, and certificate/hash parsers.

This is not equivalent to completing the full desktop acceptance protocol.
No packaged application was installed and driven through native dialogs while
the network was isolated. Native picker cancellation, offline restart,
complete legacy integrity/index-loss fixtures, several specified failure-
injection cases, and workflow-upgrade reevaluation remain open. No ATP was
moved to `completed/`.

## Reproducible commands and evidence

The following checks were actually executed from the repository root unless a
different working directory is shown:

```sh
python tools/control.py doctor
python tools/control.py test --suite all --report all
cd src-tauri
cargo test --locked
cd ../frontend
npm test
npm run build
npm audit --omit=dev
cd ..
python tools/control.py build web
python tools/control.py build desktop --target linux --bundles deb,rpm
python tools/control.py release check
```

Native Rust compilation and tests were executed in the Linux host environment
because the development Flatpak sysroot has an incompatible `libm` linker
script. This is an environment constraint, not a product test failure.

| Check | Actual result | Evidence |
| --- | --- | --- |
| Full template suite | `OK`; tools 176 passed/21 skipped, schema passed, frontend passed, Tauri passed; disabled backend/database/PostgreSQL suites correctly skipped; Playwright not configured | [Final full-suite report](../../.report/test-report-20260813-144834-suite-all-ok.md) |
| Frontend | 3 files, 19 tests passed | Final full-suite report above |
| Rust/Tauri | 26 tests passed in the final host run; structure and `cargo check` passed | Final full-suite report above and `cargo test --locked -- --list` output |
| Frontend production build | TypeScript and Vite production build passed | Command output recorded during this execution |
| Production dependency audit | 0 vulnerabilities | `npm audit --omit=dev` command output |
| General doctor | Overall status `OK`; backend explicitly disabled | Doctor command output |
| Web package | Build passed | [Web ZIP](../../.dist/web/suno-documentation-manager-web.zip) |
| Linux desktop packages | DEB and RPM build passed | [Bundle directory](../../src-tauri/target/release/bundle/) |
| Release check | All product checks passed; expected warnings for no Git repository and unsigned packages | Release-check command output |
| Visual smoke review | Responsive German welcome view inspected in Chromium; browser-demo marker visible | Execution observation; no retained screenshot artifact |

Artifact digests:

```text
422531b8d908fcfe261f79ed71097f53cf38c04c7db672d8ed012249a23739f7  .dist/web/suno-documentation-manager-web.zip
3c464ff7ea5e92130fb7d5638b78c59c3ee89ef07bbb6aee2199915ae674e9ac  src-tauri/target/release/bundle/deb/Suno Documentation Manager_0.1.0_amd64.deb
10ac97927c8d3af5541181c8bae2cffe8ba0d45adb73725316f350562a526296  src-tauri/target/release/bundle/rpm/Suno Documentation Manager-0.1.0-1.x86_64.rpm
```

## ATP execution matrix

`PASS` below means the complete wording of the listed step is supported by an
executed automated check or the recorded visual/static review. A step stays
`NOT RUN` when only part of a compound expectation was covered.

| ATP | PASS steps | NOT RUN steps | Result and precise remaining gap |
| --- | --- | --- | --- |
| [ATP-0001](../atp/active/ATP-0001-workspace-creation-and-loading.md) | 1, 3, 6 | 2, 4, 5, 7, 8, 9 | PARTIAL — native cancel/select, sibling-tree comparison, offline close/reopen, invalid target, and workspace switching remain |
| [ATP-0002](../atp/active/ATP-0002-track-creation.md) | 1, 2, 5, 6, 10 | 3, 4, 7, 8, 9 | PARTIAL — placeholder-tree inspection, snapshot assertion, editing/N/A branches, and collision sentinel remain |
| [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) | 2, 3 | 1, 4–10 | PARTIAL — partial-track non-mutation and honest unknowns pass; ambiguity, integrity recovery, adoption, cancellation, and injected failure remain |
| [ATP-0004](../atp/active/ATP-0004-document-generation.md) | 1 | 2–10 | PARTIAL — eight-file generation is integrated; golden prose, repeat determinism, snapshot/staleness, adoption, and prohibited-claim fixture checks remain |
| [ATP-0005](../atp/active/ATP-0005-artwork-evidence.md) | 2, 3, 4, 5 | 1, 6, 7, 8, 9, 10 | PARTIAL — native picker, stage combinations, content declarations, and removal action remain |
| [ATP-0006](../atp/active/ATP-0006-ai-disclosure-generation.md) | 3 | 1, 2, 4–10 | PARTIAL — network-isolated processing, pixel/placement review, reproducibility, policy branches, custom placement, and generated process documents remain |
| [ATP-0007](../atp/active/ATP-0007-sha256-generation-and-verification.md) | 1, 2, 4, 5 | 3, 6–10 | PARTIAL — complete exclusion sentinel matrix, independent `sha256sum`, changed/missing listed files, accepted regeneration, and copied-root verification remain |
| [ATP-0008](../atp/active/ATP-0008-finalization-gate.md) | 7, 8, 10 | 1–6, 9 | PARTIAL — invalid workflow fixtures, concrete blocker/status/N/A matrices, and a post-readiness native race check remain |
| [ATP-0009](../atp/active/ATP-0009-certificate-generation.md) | 1, 2, 4, 6, 8 | 3, 5, 7, 9, 10 | PARTIAL — field-by-field artifact review, JSON/data cross-check, independent verification, and certificate failure injection remain |
| [ATP-0010](../atp/active/ATP-0010-certificate-invalidation-and-revision.md) | 2, 3, 4, 5 | 1, 6–10 | PARTIAL — untouched reopen, archived-byte comparison, refinalization, and workflow-version upgrade remain |
| [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) | 2, 5, 10 | 1, 3, 4, 6–9 | PARTIAL — full round-trip, data-preserving/failed migration, metadata failure, index-loss recovery, and honest unknown-value review remain |
| [ATP-0012](../atp/active/ATP-0012-filesystem-containment.md) | 1, 3, 4, 6, 9, 10 | 2, 5, 7, 8, 11 | PARTIAL — all-command path matrix, contained-link policy/race, write/rename failure injection, and full error matrix remain |
| [ATP-0013](../atp/active/ATP-0013-end-to-end-offline-workflow.md) | 2, 3, 5–9 | 1, 4, 10, 11, 12 | PARTIAL — packaged network-isolated run, native pickers, offline restart, index-independent copied-folder review, and runtime connection/process observation remain |

## Known acceptance deviations

| ID | Scope | Observation | Follow-up required before PASS |
| --- | --- | --- | --- |
| DEV-ATP-001 | All ATPs | No immutable Git commit identifies the tested source tree. | Initialize/version the product repository or record an equivalent immutable source archive digest. |
| DEV-ATP-002 | ATP-0013 | Linux DEB/RPM packages were built but not installed or launched with network isolation. | Execute the complete packaged desktop path and retain screenshots/logs and connection observation. |
| DEV-ATP-003 | ATP-0003/0011 | A partial legacy scan/reconciliation fixture passed, but ambiguous, malformed-integrity, valid-finalized, and index-loss recovery fixtures were not executed. | Run before/after tree digests for the remaining fixture classes. |
| DEV-ATP-004 | ATP-0004/0006/0009/0012 | Workspace reopen covers two filesystem/database commit windows, but the ATP-specific document/certificate write and rename failures plus several artifact-content assertions are absent. | Add deterministic test hooks/fixtures and retain generated artifacts or golden digests. |
| DEV-ATP-005 | ATP-0006 | The protocol calls for a supported alternate disclosure placement; the executed test covers the separate bottom-right output only. | Implement/test the declared placement choices or narrow the ATP to the supported contract. |

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL — automated evidence accepted; manual/native gaps open | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |
