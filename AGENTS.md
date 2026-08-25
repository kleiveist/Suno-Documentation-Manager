<!-- AUTO-GENERATED:backlink START -->
[← Back](README.md)
<!-- AUTO-GENERATED:backlink END -->
# Coding Agent Governance

These instructions apply to every coding agent working in this repository. Follow the existing repository architecture and the policy in `config/code-quality.toml`.

## Source-size limits

1. Never create or extend a handwritten source file beyond 900 code lines. Blank lines and comment-only lines do not count toward this limit.
2. Treat 900 code lines as a hard maximum, never as a target. A file with 900 code lines is permitted; a file with 901 is an error.
3. Evaluate every file above 600 code lines for a cohesive split before extending it.
4. Normally refactor a file above 750 code lines before adding significant functionality.
5. Respect the configured limits for functions, classes, complexity, nesting, and parameter count as well as the file limit. Passing the file-size check alone does not make code clean.

## Architecture and implementation

6. Existing repository architecture takes precedence over implementation convenience. Do not invent empty layers or replace established boundaries without an explicit architecture decision.
7. Prefer existing modules, contracts, and abstractions over duplicate utilities.
8. Keep adapters, Tauri commands, and composition roots thin. They validate or translate input, delegate work, and translate output.
9. Put business rules and reusable product behavior in the appropriate application service or domain module, not in transport, framework, database, UI, or native-platform adapters.
10. Keep domain code framework-neutral. Depend on domain-owned contracts at infrastructure boundaries.
11. Preserve the product-owned SQLite, workspace, track, evidence, hashing, certificate, and document-format contracts unless a separately approved data or format migration explicitly changes them.

## Quality policy

12. Never disable or weaken lint, formatting, type, architecture, test, or quality rules merely to complete a task or make CI pass.
13. Do not add inline quality-ignore comments as a substitute for a policy decision.
14. Do not add a quality exception without a specific, documented architectural reason and an expiry date. Prefer a path-aware generated-output exclusion for generated files.
15. Resolve a genuine violation by refactoring when practical. Do not hide existing violations behind broad excludes or exceptions.

## Required verification

16. Run `python tools/control.py quality` after structural code changes and before hand-off.
17. Run the relevant automated tests after the quality gate. Expand to the full applicable suite when a change crosses subsystem boundaries.
18. Report any quality finding or test failure that remains unresolved; do not describe a partial or suppressed result as passing.

See `docs/def/code-quality.md` for the complete thresholds, rule catalog, architecture boundaries, exception format, and CI behavior.
