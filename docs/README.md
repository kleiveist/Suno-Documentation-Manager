<!-- AUTO-GENERATED:backlink START -->
[← Back](index.md)
<!-- AUTO-GENERATED:backlink END -->
# Documentation Standard

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-13 |
| Audience | Contributors, reviewers, and acceptance owners |
| Related ATP | N/A — repository-wide standard |

## Purpose

This file defines the mandatory rules for every document in the repository. Documentation is created or updated with the related code change and is part of the Definition of Done.

## Directory ownership

| Path | Content | Primary audience |
| --- | --- | --- |
| `README.md` | Product overview, quick start, and central commands | Everyone |
| `docs/def/` | Stable definitions, architecture, and domain models | Development and architecture |
| `docs/dev/` | Requirements, implementation notes, and migration plans | Development |
| `docs/usr/` | Task-oriented user guides | Users |
| `docs/tools/` | Operations, build, release, and tooling reference | Development and operations |
| `docs/tools/tauri/` | Platform-specific desktop guidance | Desktop development |
| `docs/atp/` | Acceptance test plans and protocols | Development, QA, and acceptance owners |

Empty content directories contain only `.gitkeep` until a real document is required. Do not add example documents that could be mistaken for current project documentation.

## Mandatory rules

1. All documentation is written in English. This includes the root README, templates, ATPs, generated navigation labels, examples, and operational guides.
2. A document answers one clearly stated question and has exactly one primary audience.
3. File names use English `kebab-case`, for example `release-process.md`.
4. Each authored page starts with an unambiguous H1 title and a metadata table containing status, owner, and last review date.
5. Allowed status values are `Draft`, `Active`, `Deprecated`, and `Archived`.
6. Statements describe the current state. Planned changes are explicitly marked as plans and reference a requirement or ATP.
7. Commands must be copyable and work from the documented working directory.
8. Links are relative to the current document. Update incoming and outgoing links after moves or renames.
9. Prefer Mermaid for diagrams. A diagram supplements the written explanation and does not replace it.
10. Secrets, real personal data, credentials, and local absolute paths do not belong in documentation.
11. Architecture and API changes update the affected documentation, tests, and ATP in the same change.

PyGitIndex navigation pages and generated blocks are exempt from the metadata-table requirement because they contain navigation only. Do not edit content between `AUTO-GENERATED` markers manually.

## Required structure

Copy [DOCUMENT-TEMPLATE.md](DOCUMENT-TEMPLATE.md) for a new authored page. Optional sections may be removed, but these elements remain mandatory:

- Title
- Status, owner, and last review date
- Purpose
- Scope
- Details or procedure
- Verification
- Related documents

## Writing style

- Use short, verifiable sentences and concrete verbs.
- Define a technical term when it first appears and use it consistently afterward.
- Use English for prose, headings, diagram labels, table labels, examples, and placeholder descriptions.
- Mark examples as examples and wrap placeholders in angle brackets, such as `<project-name>`.
- Avoid relative timing such as “soon” or “currently” without an explicit date.

## Code, API, and configuration

Every code block has a language identifier. API documentation includes at least method, path, input, successful output, and error cases. Configuration documentation identifies the name, default, allowed values, security impact, and an example.

## Generated navigation

Use the project wrapper for the system-installed PyGitIndex script:

```sh
python tools/control.py docs index --dry-run
python tools/control.py docs index
```

The wrapper locates `PyGitIndex.py`, regenerates directory indices and backlinks, and normalizes generated navigation labels to English. Set `PYGITINDEX_PATH` or pass `--script <path>` when the script is stored outside a known location.

Always review the dry-run before updating navigation. Run the update after adding, moving, renaming, or removing a Markdown file.

## Review and maintenance

The code reviewer verifies:

- Does the documentation match the implemented behavior?
- Do commands and links work?
- Is the change supported by tests or an ATP?
- Was obsolete guidance replaced instead of merely supplemented?
- Is the `Last review` date correct?
- Is every authored sentence in English?

A document becomes `Deprecated` when it no longer describes the preferred solution. It becomes `Archived` when it serves only as historical evidence. Active pages must not rely on archived content as their only source.

## Documentation workflow

1. Identify the audience and select the correct directory.
2. Copy [DOCUMENT-TEMPLATE.md](DOCUMENT-TEMPLATE.md).
3. Link the requirement, architecture decision, or ATP.
4. Update documentation alongside the implementation.
5. Verify commands, links, examples, and English language consistency.
6. Set the owner and review date.
7. Preview and regenerate the PyGitIndex navigation.
8. Review and commit documentation with the code.

## Verification

```sh
python tools/control.py docs index --dry-run
python tools/control.py test --suite tools
```

Search Markdown files for obsolete non-English content before hand-off.

## Related documents

- [Tooling Guide](tools/tooling.md)
- [Framework Architecture](def/architecture.md)
- [ATP Workflow](atp/README.md)
