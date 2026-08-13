<!-- AUTO-GENERATED:backlink START -->
[← Back](atp.md)
<!-- AUTO-GENERATED:backlink END -->
# ATP-<ID>: <Acceptance title>

| Field | Value |
| --- | --- |
| Status | planned |
| Owner | <person or team> |
| Created | YYYY-MM-DD |
| Executed | Not yet executed |
| Requirement | <relative link or issue ID> |
| Tested commit/build | <commit SHA, tag or artifact ID> |
| Environment | <OS, browser, desktop target and service versions> |

## Objective

<State the user-visible capability or quality property being accepted.>

## Scope

### Included

- <Behavior included in this acceptance>

### Excluded

- <Explicit non-goal>

## Risks

| Risk | Impact | Mitigation or test focus |
| --- | --- | --- |
| <risk> | <impact> | <mitigation> |

## Preconditions

- [ ] Required dependencies are installed.
- [ ] The test environment and build are identified.
- [ ] Required services are healthy.
- [ ] Test data is prepared and contains no sensitive production data.

## Test data

| ID | Description | Source or setup |
| --- | --- | --- |
| TD-01 | <data set> | <reproducible setup> |

## Acceptance steps

| Step | Action | Expected result | Actual result | Status | Evidence |
| --- | --- | --- | --- | --- | --- |
| 1 | <action> | <observable outcome> | Not run | NOT RUN | — |

## Automated checks

```sh
<command>
```

<Link the command output, CI run or generated report.>

## Deviations

| ID | Description | Severity | Owner | Follow-up | Status |
| --- | --- | --- | --- | --- | --- |
| DEV-01 | <deviation or `None`> | <severity> | <owner> | <issue/link> | <open/closed> |

## Result

- Overall result: `PENDING`
- Summary: <explain the acceptance decision>
- Residual risks: <remaining accepted risks or `None`>

## Sign-off

| Role | Name | Decision | Date |
| --- | --- | --- | --- |
| Acceptance owner | <name> | PENDING | YYYY-MM-DD |
