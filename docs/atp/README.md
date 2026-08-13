<!-- AUTO-GENERATED:backlink START -->
[← Back](atp.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP Workflow

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Development, QA, and acceptance owners |
| Related ATP | N/A — defines the ATP process |

## Purpose

In this template, **ATP** means **Acceptance Test Plan/Protocol**. An ATP file first defines the planned acceptance work. During execution, the same file records actual results, deviations, evidence, and approval.

## Directory structure

```text
docs/atp/
├── README.md
├── ATP-TEMPLATE.md
├── planned/
├── active/
└── completed/
```

| Directory | Meaning |
| --- | --- |
| `planned/` | Scope and expected results are defined, but execution has not started. |
| `active/` | Acceptance is in progress or unresolved deviations remain. |
| `completed/` | All mandatory steps were executed and the result was approved. |

## File name and ID

Use `ATP-<four-digit-ID>-<short-slug>.md`, for example `ATP-0007-user-login.md`. Never reuse an ID. Move the file when its status changes, but do not change its ID or file name.

## Workflow

1. Copy `ATP-TEMPLATE.md` to `planned/ATP-<ID>-<slug>.md`.
2. Define the requirement, scope, risks, prerequisites, test data, and expected results before implementation acceptance.
3. Verify that every acceptance criterion maps to at least one test step.
4. Move the file to `active/` when execution begins.
5. Record the actual result, status, and evidence immediately for each test step.
6. Document every deviation with an owner and follow-up action. A failed mandatory step prevents `completed` status.
7. After successful retesting, record the final result and sign-off, then move the file to `completed/`.
8. Run `python tools/control.py docs index` after moving the ATP so navigation remains accurate.

## Test-step statuses

Use only these values:

- `NOT RUN`: execution has not started
- `PASS`: the expected result was fully achieved
- `FAIL`: the expected result was not achieved
- `BLOCKED`: a documented dependency prevents execution
- `N/A`: a reviewer confirmed that the step is not applicable; a reason is mandatory

## Evidence

Evidence must be discoverable and understandable to a reviewer. Suitable evidence includes relative paths to reports or screenshots, CI run IDs, reproducible commands, relevant log excerpts, and external-system versions. Never include secrets, credentials, or real personal data.

## Completion criteria

An ATP may move to `completed/` only when:

- every mandatory step is `PASS` or has a justified `N/A` status;
- no open deviation blocks acceptance;
- the environment and tested commit or build are identified;
- automated checks are referenced, or justified manual checks are documented;
- the overall result and sign-off are complete; and
- documentation navigation has been regenerated.

Never use the template file itself as an executed ATP.

## Verification

```sh
python tools/control.py test --suite all --report
python tools/control.py docs index --dry-run
```

## Related documents

- [ATP Template](ATP-TEMPLATE.md)
- [Documentation Standard](../README.md)
- [Tooling Guide](../tools/tooling.md)
