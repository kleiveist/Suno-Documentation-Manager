<!-- AUTO-GENERATED:backlink START -->
[← Back](dev.md)
<!-- AUTO-GENERATED:backlink END -->
# Legacy track import and managed-document adoption

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Developers implementing or reviewing legacy import |
| Related ATP | [ATP-0003: Legacy track import](../atp/active/ATP-0003-legacy-track-import.md) |

## Purpose

This document defines a conservative import algorithm for existing track folders. It answers how the application discovers useful historical evidence without overwriting files, inventing missing facts, or treating an unverified folder as finalized.

## Scope

### Included

- workspace scan and track-candidate detection;
- known directory, document, evidence, hash, and certificate discovery;
- mapping confidence and `NOT VERIFIED` behavior;
- explicit recoverable removal of indexed legacy evidence;
- explicit managed-document adoption and backup; and
- recovery from an absent SQLite index.

### Excluded

- automatic repair of malformed media or documents;
- inferring legal rights, authorship, dates, or account history from filenames;
- silently normalizing or moving a historical folder;
- deleting duplicate or unknown files; and
- importing from a remote service.

## Safety invariants

Legacy scanning is read-only with respect to every candidate track directory. It may create or reconcile a local SQLite index record so the candidate can be reviewed in the application. The implementation preserves these invariants:

1. Never overwrite, rename, move, or delete a discovered file during scan.
2. Treat a filename-derived evidence role only as an unverified classification, never as confirmed historical provenance.
3. Never mark a historical hash or certificate valid without verification.
4. Never follow a symbolic link outside the canonical workspace root.
5. Never turn an unknown answer into `No` solely to make a branch N/A.
6. Never write a managed document over existing content before preview, confirmation, and backup.

## Candidate detection

`scan_workspace` ignores the reserved `.suno-doc/` area and considers direct workspace child directories as track candidates. A directory gains confidence when it contains one or more known structures:

```text
01_RELEASE/
02_SUNO/
03_DOCUMENTATION/
04_LICENSES/
05_ARTWORK/
06_CERTIFICATE/
```

A candidate name becomes its initial display title while the track remains marked as a legacy import. The scan records the exact relative folder path. Users must review the title and every missing fact before the current workflow can pass.

Unknown sibling files and directories remain in place. They can be shown as unclassified evidence candidates but are not silently copied into a known role.

## Discovery mapping

| Discovered item | Version 0.1 scan result | Required follow-up |
| --- | --- | --- |
| Known managed-document path | Path is listed; an exact template header distinguishes managed content from a collision. | Preview and explicitly adopt any unmanaged destination before generation. |
| `03_DOCUMENTATION/SHA256SUMS.txt` | Presence is reported, but no historical PASS state is inferred. | Regenerate and verify after the imported facts and evidence are reviewed. |
| Known file in `01_RELEASE/` or `02_SUNO/` | A bounded filename/location rule proposes a role; the item is indexed with `indexed_legacy` provenance and remains `NOT VERIFIED`. | Explicitly verify the present bytes and review whether the role is truthful. |
| Artwork naming convention in `05_ARTWORK/` | The role is proposed and remains `NOT VERIFIED`. | Confirm the artwork process and all conditional facts. |
| Other contained file outside `.archive/` and `06_CERTIFICATE/` | Indexed as `other`, still `NOT VERIFIED`. | Reclassify through an explicit fresh import when a mandatory role is needed. |
| Existing `06_CERTIFICATE/` content | Preserved untouched and excluded from evidence inference. | Version 0.1 does not silently reinstate a historical finalized database state. Review it independently before starting a new managed revision. |

Conflicting documents, duplicate candidate roles, malformed manifests, unsupported schema versions, absolute paths, and paths escaping the track root are reported. They are not resolved automatically.

## Import states

| State | Meaning | Finalization effect |
| --- | --- | --- |
| Discovered | A track candidate exists but is not indexed. | Cannot finalize |
| `INCOMPLETE` presentation | Some known structure exists, but mandatory current facts or evidence are missing. | Cannot finalize |
| `NOT VERIFIED` step | A historical claim or artifact exists without sufficient validation. | Blocks the mandatory step |
| Verified working track | Confirmed facts and evidence are indexed, but the current workflow gate has not passed. | Evaluate normally |
| Verified finalized snapshot | Manifest, certificate, workflow version, and hashes all validate. | Can be presented as finalized without rewriting it |

`NOT VERIFIED` is a workflow-step result, not permission to fill missing data. A user must confirm a fact or provide evidence before it can pass.

## Scan algorithm

For each direct candidate directory:

1. Canonicalize the workspace and candidate roots.
2. Reject or isolate a candidate that resolves outside the workspace.
3. Walk only the contained tree without following an escaping symbolic link.
4. List known documents, report hash-list presence, and classify contained evidence candidates by bounded filename, extension, and location rules.
5. Add or reconcile a legacy SQLite record; do not modify the candidate directory.
6. Store discovered evidence with `indexed_legacy` provenance, its byte hash, and an explicit historical-provenance `NOT VERIFIED` reason.
7. Reevaluate the selected `suno-track` workflow and display exact missing requirements.
8. On later scans, add newly discovered files without duplicating already indexed paths.
9. Require explicit verification for evidence before it can satisfy a role.
10. Require explicit confirmation before copying the complete current workspace profile into the legacy track snapshot.

The scan proposal is reproducible and serializable, but it does not contain an absolute local path in portable output.

## Removing indexed legacy evidence

Scan itself remains read-only. After review, a user can explicitly remove one indexed legacy evidence item from the managed track state. This action does not permanently delete the observed file:

1. Resolve the indexed root-relative path inside the track and reject a symbolic link or non-regular file.
2. Create a unique `.archive/removals/<removal-id>/` directory.
3. Move a present file into that directory without changing its bytes.
4. Write `removal.json` with schema version, removal and track IDs, time, reason, original relative path, and the complete evidence metadata including `indexed_legacy` provenance.
5. Remove the SQLite evidence row and persist the changed track state.
6. Roll back the file and index change when a recoverable commit step fails.

If the indexed file was already missing, the action still writes the removal record before de-indexing. The original path is absent after success, and `.archive/` is excluded from discovery, so a later scan does not undo the explicit removal. Version 0.1 provides the preserved bytes and metadata for manual recovery; it does not provide an automatic restore action.

## Managed-document adoption

An existing document is unmanaged unless it has a recognized managed-template marker and compatible template version. To replace an unmanaged document with a generated version:

1. Display the current relative path and existing content or a safe preview.
2. Display the generated candidate and material differences.
3. Explain that adoption will archive the existing file.
4. Require explicit confirmation for that document or an explicitly listed set.
5. Copy the original into a unique contained location below `.archive/` and verify the backup.
6. Write the managed document through a temporary sibling file and atomic rename.
7. Record the backup path, template version, adoption time, and resulting hash.
8. Reevaluate document freshness and integrity.

If backup or atomic write fails, preserve the original and return a controlled error. The index must not claim adoption succeeded.

## Recovering after index loss

When `.suno-doc/workspace.sqlite` is absent, opening the existing workspace creates a fresh local index and a subsequent scan can rediscover track directories and contained evidence. The scan rebuilds only observable paths, sizes, current byte hashes, and conservative role proposals. Mutable UI state, global defaults, unexported facts, and a prior finalized database state cannot be inferred and remain unset or blocked.

Existing certificate artifacts remain portable and independently inspectable, but version 0.1 does not automatically convert them into a `FINALIZED` SQLite record after index loss. The recovered track is active/legacy with exact blockers until facts and evidence are reviewed. This conservative behavior avoids claiming that an unknown database snapshot was reconstructed.

Workspace recovery does not move historical `06_CERTIFICATE/` content merely because the recovered SQLite record is non-finalized. Certificate quarantine is enabled only by a matching `.archive/finalization-in-progress.json` marker written by the application immediately before its own certificate publication. Without that marker, historical certificate bytes remain exactly where the scan found them.

## Requirements and ATP mapping

| Requirement | Acceptance criterion | Acceptance plan |
| --- | --- | --- |
| `REQ-LEG-001` | Scan detects known structures and reports unknowns without changing the candidate tree. | [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) |
| `REQ-LEG-002` | Missing historical facts remain unknown or `NOT VERIFIED`; they are never invented. | [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) |
| `REQ-LEG-003` | Existing documents require preview, confirmation, verified archive backup, and atomic write before adoption. | [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) |
| `REQ-LEG-004` | Absolute, traversal, and escaping symbolic-link manifest paths are rejected. | [ATP-0012](../atp/active/ATP-0012-filesystem-containment.md) |
| `REQ-LEG-005` | Index loss leaves portable certificate files untouched; scan recovers only observable facts and clearly reports the remaining blockers. | [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md) |
| `REQ-LEG-006` | Explicitly removed indexed legacy evidence is preserved below `.archive/removals/` with metadata and is not re-added by a later scan. | [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md) |

## Verification

Automated tests use temporary workspaces and copy disposable fixtures. Fixtures cover an empty candidate, partial known tree, unmanaged document, duplicate role, valid and invalid hash list, valid and malformed manifest, absolute path, traversal, symbolic-link escape where supported, and valid finalized snapshot.

Verification commands, run from the repository root, are:

```sh
python tools/control.py test --suite tauri
python tools/control.py test --suite all --report
```

Reviewers compare pre-scan and post-scan candidate-tree hashes to prove that discovery did not mutate track files. Execution evidence and open manual checks are recorded in [ATP-0003](../atp/active/ATP-0003-legacy-track-import.md), [ATP-0011](../atp/active/ATP-0011-local-persistence-and-recovery.md), and the [acceptance report](acceptance-report.md).

## Risks and limitations

- Filename conventions are hints and can produce ambiguous candidates; user confirmation remains necessary.
- An old folder may lack enough portable information to recover a prior workflow result.
- A malformed historical document can be preserved but not safely parsed.
- Existing symbolic links are rejected, but the path-based implementation does not close a same-user concurrent symbolic-link swap race; this remains an explicit version 0.1 threat-model and ATP limitation.

## Related documents

- [Persistence and recovery](../def/persistence.md)
- [Track documentation model](../def/track-documentation-model.md)
- [Workflow model](../def/workflow-model.md)
- [Getting started](../usr/getting-started.md)
- [ATP-0003: Legacy track import](../atp/active/ATP-0003-legacy-track-import.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-13 | Added indexed-legacy provenance, recoverable evidence removal, and marker-scoped certificate recovery. | Project team |
| 2026-08-13 | Aligned conservative indexing, rescan reconciliation, and explicit profile adoption with version 0.1. | Project team |
| 2026-08-13 | Defined conservative legacy discovery, recovery, and document adoption. | Project team |
