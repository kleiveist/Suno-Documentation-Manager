<!-- AUTO-GENERATED:backlink START -->
[← Back](active.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-0005: Artwork evidence and content declarations

| Field | Value |
| --- | --- |
| Status | active |
| Owner | Product team |
| Created | 2026-08-13 |
| Last review | 2026-08-13 |
| Executed | 2026-08-13 — partial automated execution |
| Requirement | [`REQ-EVD-001`, `REQ-ART-001`](../../def/track-documentation-model.md#requirements-and-atp-mapping) |
| Tested commit/build | Product `0.1.0`; unversioned source tree; Linux package digests in the central report |
| Environment | Linux `7.0.8-1-cachyos` `x86_64`; temporary native image/evidence fixtures |

## Purpose

This plan verifies native artwork-evidence import, stage naming, collision protection, and conditional real-person, real-event, and trademark/logo declarations.

## Objective

Accept artwork evidence when real files are copied into contained roles, originals remain unchanged, only actual production stages are required, and positive content declarations request factual notes without producing legal conclusions.

## Scope

### Included

- AI original, AI-edited, human-edited, and final artwork roles;
- native file selection, type validation, safe copy, size, and SHA-256 metadata;
- naming convention and optional-stage behavior;
- destination collision handling; and
- three conditional content checks.

### Excluded

- visible disclosure image processing, covered by ATP-0006;
- legal review of depicted content; and
- remote image services.

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| Import moves or changes the source | Original evidence loss | Compare source path and hash before and after import |
| Role collision overwrites prior evidence | Evidence loss or false provenance | Import two different files to the same role |
| Every artwork stage is forced | User fabricates nonexistent process | Test minimal AI-only and human-only processes |
| Positive content answer becomes a legal decision | Misleading documentation | Inspect step state and generated factual fields |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] A disposable track exists.
- [ ] Test images are valid, synthetic, and contain no real personal data.
- [ ] Expected source hashes and sizes are recorded.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | AI original | Valid PNG with deterministic synthetic pixels |
| TD-02 | Different AI candidate | Valid PNG with same proposed role and filename collision |
| TD-03 | Human-edited and final files | Valid PNG and JPEG synthetic images |
| TD-04 | Invalid image | Text bytes with a `.png` extension |
| TD-05 | Content declarations | Separate `No` and `Yes` answers with neutral synthetic notes |

## Acceptance steps

| Step | Requirement | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `REQ-EVD-001` | Import TD-01 through the native picker as AI artwork original. | A contained copy is created with role, relative path, validated type, size, and SHA-256 metadata. | Not run | NOT RUN | — |
| 2 | `REQ-EVD-001` | Compare TD-01 before and after import. | The source remains present and byte-identical. | Native import retained the source bytes and created a byte-identical managed copy. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 3 | `REQ-ART-001` | Inspect the proposed managed name. | It follows `<track>_AI_ORIGINAL.png` and does not alter the original source name or file. | The managed path was exactly `05_ARTWORK/My-Track_AI_ORIGINAL.png`. | PASS | Rust `artwork_import_uses_documented_role_naming` |
| 4 | `REQ-EVD-001` | Attempt to import TD-02 to the occupied role/destination. | The application reports a conflict and neither existing destination nor source is overwritten or removed. | Duplicate import returned `Collision`; source and destination bytes remained intact. | PASS | Rust `evidence_import_validates_type_preserves_source_and_rejects_collision` |
| 5 | `REQ-EVD-001` | Attempt TD-04 as artwork. | Type validation rejects the file with a controlled error and creates no evidence record. | A `.png` containing plain-text bytes was rejected by signature validation before copy/indexing. | PASS | Rust `artwork_import_uses_documented_role_naming`; [central report](../../dev/acceptance-report.md) |
| 6 | `REQ-ART-001` | Model an AI original followed directly by a final output. | AI-edited and human-edited intermediate stages are not required. | Not run | NOT RUN | — |
| 7 | `REQ-ART-001` | Model the full TD-01/TD-03 production sequence. | Present stages use `AI_ORIGINAL`, optional `AI_EDITED` where supplied, `EDITED`, and `FINAL` roles in order. | Not run | NOT RUN | — |
| 8 | `REQ-ART-001` | Answer `No` to each TD-05 content check. | Each branch ends without unrelated follow-up requirements. | Not run | NOT RUN | — |
| 9 | `REQ-ART-001` | Answer `Yes` separately for real person, real event, and trademark/logo. | A factual note and configured evidence become applicable; no legal result is generated. | Not run | NOT RUN | — |
| 10 | `REQ-EVD-001` | Remove an imported disposable role through the product action. | Only the explicitly selected managed evidence is affected according to confirmation; the original source remains untouched and workflow reevaluates. | Not run | NOT RUN | — |

## Automated checks

```sh
cd src-tauri
cargo test evidence_import_validates_type_and_rejects_collision
cd ../frontend
npm test -- --run src/domain/workflow.test.ts
```

Expected Rust evidence is `tests::evidence_import_validates_type_and_rejects_collision`. Expected Vitest evidence includes `conditional fields > shows AI/artwork follow-ups conditionally`.

## Verification

Evidence includes source and destination digests, relative paths, role metadata, collision output, invalid-type output, and branch screenshots. Do not attach real artwork or personal depictions.

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | Native picker use, optional/full stage sequences, content-declaration branches, and product removal remain unexecuted. | Medium | Product team | Execute steps 1 and 6–10 with retained UI and file metadata evidence. | open |

## Result

- Overall result: `PARTIAL`
- Summary: Steps 2–5 passed; six mandatory steps remain `NOT RUN`.
- Residual risks: Native picker behavior, content declarations, removal, and platform decoder differences are not accepted yet.

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Automated acceptance executor | Codex | PARTIAL | 2026-08-13 |
| Product acceptance owner | — | PENDING | — |

## Related documents

- [Track documentation model](../../def/track-documentation-model.md)
- [Getting started](../../usr/getting-started.md)
- [ATP-0006: AI disclosure generation](ATP-0006-ai-disclosure-generation.md)
