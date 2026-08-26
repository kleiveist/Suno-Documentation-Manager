<!-- AUTO-GENERATED:backlink START -->
[← Back](def.md)
<!-- AUTO-GENERATED:backlink END -->
# Code quality and architecture governance

| Field | Value |
| --- | --- |
| Status | Active |
| Owner | Project team |
| Last review | 2026-08-23 |
| Audience | Contributors, reviewers, architects, and coding agents |
| Related ATP | N/A — repository-wide engineering policy |

## Purpose

This document defines the repository's enforceable Clean Code, source-size, and architecture policy. The central policy is `config/code-quality.toml`; `python tools/control.py quality` is the public local and CI entry point. Stable rule IDs make every repository-owned finding identifiable in text and JSON reports, tests, exceptions, and CI logs.

The governance system has four goals:

- stop unreviewable handwritten source files and oversized functions from growing unchecked;
- detect risky complexity and dependency direction before it becomes structural debt;
- give developers and coding agents actionable warnings before a hard limit is reached; and
- use Ruff, ESLint, Prettier, TypeScript, rustfmt, Clippy, and Cargo for language-specific checks instead of reimplementing them.

It is a guardrail around the existing architecture. It does not create speculative layers, require one module per small function, or replace design review and tests.

## Clean Code is more than a LOC limit

**A file below 900 code lines is not automatically clean or well-architected.** The 900-line limit is an absolute ceiling, not a design target. A 200-line module can still be difficult to change if it mixes responsibilities, contains deeply nested decisions, bypasses a layer boundary, duplicates an existing abstraction, or cannot be tested independently.

Review code along all of these dimensions:

- file, function, and class size;
- cyclomatic complexity and nesting;
- function parameter count;
- cohesion and separation of responsibilities;
- dependency direction and framework boundaries;
- duplication and use of existing contracts;
- testability, readability, and maintainability; and
- whether routers, controllers, adapters, and composition roots remain thin.

LOC findings are an early signal and a hard safety boundary. They are not a substitute for engineering judgment.

## Severity and gate behavior

| Severity | Meaning | Effect on the quality exit code |
| --- | --- | --- |
| `INFO` | Context that requires no corrective action | None |
| `WARNING` | Review and consider refactoring before the code grows | None |
| `STRONG_WARNING` | An explicit refactoring candidate close to a hard boundary | None by itself |
| `ERROR` | A policy violation that must be resolved | Fails the gate |

The normal development command exits with `0` when all checks run successfully and there is no unsuppressed `ERROR`. Warnings and strong warnings remain visible but do not make ordinary CI red. Release Validation uses `quality --release`, which also returns non-zero for an unsuppressed `STRONG_WARNING`. A narrow, valid, expiring exception records the required release decision without weakening the hard limits. An unsuppressed `ERROR`, invalid configuration, missing required tool, or failed lint, type, format, compiler, or architecture check always produces a non-zero exit code.

## Exact measurement boundaries

### Files

| Measurement | PASS | `WARNING` | `STRONG_WARNING` | `ERROR` |
| --- | ---: | ---: | ---: | ---: |
| Code lines per handwritten source file | 0–600 | 601–750 | 751–900 | 901 or more |
| Physical lines per source file | 0–1200 | 1201 or more | Not used | Not used |

Exactly 900 code lines are permitted and produce a strong warning without failing the gate. Exactly 901 code lines produce `CQ001` at `ERROR` severity and fail the gate. A `CQ001 ERROR` is unsuppressible: even an otherwise valid matching exception remains visible and cannot change the failing exit code. A handwritten source file must never use an exception to turn 900 into a target or to bypass the 901-line failure.

Physical lines include code, comments, and blank lines. The physical-line warning exposes files whose review cost is hidden by large comment or whitespace sections; it is not a hard failure in policy version 1.

### Functions, classes, and control flow

| Measurement | PASS | `WARNING` | `STRONG_WARNING` | `ERROR` |
| --- | ---: | ---: | ---: | ---: |
| Code lines per function or method | 0–50 | 51–80 | 81–120 | 121 or more |
| Code lines per class | 0–300 | 301–500 | 501–700 | 701 or more |
| Cyclomatic complexity | 1–10 | 11–15 | 16–20 | 21 or more |
| Nesting depth | 1–3 | 4 | 5 | 6 or more |
| Function or method parameters | 0–5 | 6–8 | 9–10 | 11 or more |

A nesting depth of zero also passes. Parameters at the boundary are intentional: eight parameters are a warning, while nine parameters are a strong warning. Related parameters should normally become a request, configuration, DTO, or domain object rather than being hidden behind a longer signature.

### Code LOC definition and language coverage

A code line is one physical line containing at least one source token. It is counted once even if it contains several statements. Imports, declarations, functions, methods, classes, type definitions, executable statements, and source configuration inside a code file count. Blank lines and comment-only lines do not count. A line containing both code and a trailing comment counts as code.

The repository scanner supports `.py`, `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, and `.rs`. Python tokenization and C-style comment/string handling prevent blank lines and single-line or multiline comment-only content from inflating code LOC. Content lines in ordinary or raw multiline strings remain code, including Rust strings whose content begins with comment markers.

Reliable scope and complexity analysis is intentionally language-specific:

| Language area | Automated analysis |
| --- | --- |
| Python | File and physical LOC, AST-based function/class LOC, Ruff complexity, nesting, parameter count, lint, and format check |
| Frontend `.js`, `.jsx`, `.ts`, and `.tsx` | File and physical LOC, TypeScript-AST class LOC, ESLint function LOC, complexity, nesting, parameter count and lint, TypeScript compiler check, and Prettier check |
| Tauri/Rust | File and physical LOC; repository-owned function and class-like LOC, cyclomatic complexity, nesting, and parameter metrics; rustfmt; Clippy warnings plus defensive hard limits for function length and parameters; and locked Cargo checking and testing across all targets and features |

TypeScript-AST function and class symbols include their lexical namespace, class, and object containers, for example
`Admin.OrderView.render`. Assigned anonymous class expressions use their variable or property name. Findings and symbol-scoped
exceptions therefore do not conflate same-named declarations in different containers. ESLint metric findings prefer that AST
symbol and use the unqualified function or method name parsed from an ESLint message only when no matching AST scope is
available. Ruff metric findings follow the same AST-first rule for qualified Python function and method symbols.

The Rust metric analyzer uses Syn 2.0.119 rather than token-pattern guesses. The parser and metric collector are compiled to a
bundled `wasm32-wasip1` module and executed by the pinned Wasmtime 47.0.1 tooling dependency. The 535,787-byte release module
has SHA-256 `7a27cb02a9392b62c487b4ce73a03524d7260b804c0c49eb4520ceb1a1cacfd8`. Its provenance binds the exact Rust 1.97.1 compiler
commit, target and dependency versions, and a source digest over `build.py`, `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, and `src/**/*.rs`. The host verifies the artifact against this versioned provenance before execution
and grants it no filesystem preopens or inherited environment. The analyzer
treats named functions, methods, closures, and async blocks as isolated function scopes, and `struct`, `enum`, `union`,
`trait`, and `impl` bodies as class-like scopes. Module, implementation, function, and generated closure locations qualify
symbols so separate same-named scopes cannot share an exception accidentally. Cyclomatic complexity starts at one and adds a
path for parsed `if`, `while`, `for`, and `loop` expressions, logical `&&` and `||` paths (including Rust 2024 let chains),
the try operator, each `match` arm, and each match guard. `let ... else` alternatives also add a path. Nesting follows parsed
control-flow bodies, `let ... else` alternatives, and block-bodied match arms. A parsed method receiver such as `self` or
`&self` is not counted as a parameter. Nested functions, closures, async blocks, and local item declarations are not
attributed to their parent function's metrics. Unsupported verbatim syntax and malformed analyzer output fail the scan; the
only compatibility reparse is the narrowly validated Rust 2024 `safe` foreign-item form supported by rustc but represented as
verbatim by the pinned Syn version. Macro token trees are intentionally not expanded; their written lines still contribute to
enclosing file and function LOC, while compiler and Clippy checks validate the compiled expansion separately.

`.mjs` and `.cjs` receive repository file/physical LOC scanning and normal frontend tool coverage where applicable, but not the scope metrics above. If a metric cannot be derived reliably for a language construct, the system leaves it to review instead of using a fragile regular expression. That limitation is not permission to create oversized or tangled code.

## Rule catalog

### Code-quality rules

| Rule | Stable name | Scope and result |
| --- | --- | --- |
| `CQ001` | `FILE_CODE_LINES` | Applies the 600/750/900 code-LOC file boundaries; more than 900 is an unsuppressible `ERROR` |
| `CQ002` | `FILE_PHYSICAL_LINES` | Warns when a source file has more than 1200 physical lines |
| `CQ101` | `FUNCTION_LINES` | Applies the 50/80/120 function and method boundaries |
| `CQ102` | `FUNCTION_COMPLEXITY` | Applies the 10/15/20 cyclomatic-complexity boundaries |
| `CQ103` | `FUNCTION_NESTING` | Applies the 4/5 nesting boundaries; more than 5 is `ERROR` |
| `CQ104` | `FUNCTION_PARAMETERS` | Applies the 6–8/9–10 parameter warning bands; more than 10 is `ERROR` |
| `CQ201` | `CLASS_LINES` | Applies the 300/500/700 class boundaries |

### Architecture rules

| Rule | Stable name | Scope and result |
| --- | --- | --- |
| `AR001` | `INVALID_LAYER_DEPENDENCY` | Reports a configured backend or frontend layer violation, or a forbidden quality-tooling or general-tooling dependency |
| `AR002` | `DOMAIN_FRAMEWORK_DEPENDENCY` | Reports direct framework or concrete persistence dependencies from the backend domain |
| `AR003` | `CROSS_FEATURE_INTERNAL_IMPORT` | Reports a frontend feature importing another feature through an internal module instead of its public root index |
| `AR004` | `ROUTER_BUSINESS_LOGIC` | Reports direct database/SQL concerns in a FastAPI router or a route handler above 50 code lines |

Architecture violations are `ERROR` findings. The automated boundaries are detailed under [Architecture policy](#architecture-policy).

### Exception rules

| Rule | Stable name | Scope and result |
| --- | --- | --- |
| `EX001` | `INVALID_EXCEPTION` | Reports malformed, unknown, duplicate, non-specific, or otherwise invalid exception data as `ERROR` |
| `EX002` | `EXPIRED_EXCEPTION` | Reports an expired exception as `ERROR`; the original finding is no longer suppressed |

`EX001` and `EX002` cannot themselves be excepted.

## Central configuration

All thresholds, scanned extensions, excludes, architecture mappings, and exceptions live in `config/code-quality.toml`. The quality command loads and validates this file before scanning. The configuration has schema version 1 and rejects invalid TOML, missing tables, non-positive limits, inconsistent threshold ordering, duplicate values, unsafe paths, unknown layer pairs, and invalid exception entries with a clear diagnostic.

Do not duplicate governance thresholds in tool-specific files. The orchestrator derives temporary Ruff and ESLint metric rules from the central configuration. Tool-native files remain responsible for ordinary language lint and formatting policy.

An alternate policy file can be inspected with the CLI's `--config` option, but committed CI uses `config/code-quality.toml`. Lowering a limit or changing an architecture boundary is a policy change and requires the same review as an architecture change.

## Source excludes

Generated, external, dependency, cache, and build output must not distort repository measurements. The current defaults are:

| Type | Excluded values |
| --- | --- |
| Directory names | `.cache`, `.dist`, `.generated`, `.git`, `.pytest_cache`, `.venv`, `__pycache__`, `coverage`, `dist`, `generated`, `node_modules`, `target`, `vendor` |
| Exact file names | `Cargo.lock`, `package-lock.json` |
| Repository-relative path patterns | `build/**`, `backend/build/**`, `frontend/dist/**`, `src-tauri/gen/**`, `src-tauri/target/**` |

Directory-name excludes apply to matching directory components. File excludes apply to exact basenames. Path patterns are evaluated against normalized repository-relative paths.

`build` is deliberately **not** a global directory-name exclude. The path-aware patterns exclude known root/backend output while files such as `tools/tauri/build/appimage.py` remain in scope because they are handwritten source. Add a generated-output path to the narrowest appropriate path pattern; do not add a broad directory name or hundreds of exceptions merely to make the gate pass.

## Controlled exceptions

Manual exceptions are centralized in `config/code-quality.toml`; scattered `# ignore quality` comments are not the governance mechanism. A manual exception must identify one real file and one known `CQ` or `AR` rule:

```toml
[[exceptions]]
rule = "AR001"
path = "backend/app/legacy/bridge.py"
symbol = "load_legacy_record" # Optional: narrows a symbol-level finding.
reason = "Temporary compatibility bridge while the legacy provider is retired."
expires = "2027-01-01"
```

The schema and review rules are:

| Field | Requirement |
| --- | --- |
| `rule` | Required known `CQ` or `AR` ID; `EX` rules are not suppressible |
| `path` | Required exact repository-relative file path; absolute paths, `..`, and globs are rejected |
| `reason` | Required meaningful architectural reason of at least 10 characters |
| `expires` | Required ISO date in `YYYY-MM-DD` form |
| `symbol` | Optional exact function, method, or class symbol for a narrower match |

The referenced file must exist, and duplicate entries for the same rule, path, and symbol are invalid. A valid exception suppresses only a suppressible finding whose rule, path, and optional symbol match exactly. The report retains the suppressed finding, reason, and expiry for visibility. It does not rewrite the rule or lower its severity. `CQ001` warning bands may still be reviewed through the ordinary exception mechanism when justified, but a `CQ001 ERROR` above 900 code lines is never suppressed.

An entry is valid through its stated expiry date. On a later date it produces `EX002 EXPIRED_EXCEPTION` at `ERROR`, stops suppressing the original finding, and fails the gate. Resolve the violation before expiry whenever possible. Renew an exception only after a fresh architecture review; changing the date is not routine maintenance.

Use source excludes, not exceptions, for reproducible generated or third-party content. Do not add an exception for a handwritten file above 900 code LOC; the gate deliberately ignores such an entry when applying exceptions to the error finding.

Tool-native inline suppressions are not an alternative exception channel. Quality invocations make Ruff ignore `noqa`, disable ESLint inline configuration, and run Clippy hard-limit lints at `forbid` level so source attributes cannot lower them. A justified temporary exception must use the central, exact-path, expiring mechanism above.

## Architecture policy

### Backend dependency direction

The target direction follows the repository's real structure without requiring empty directories:

```text
API / router
     |
     v
application / services
     |
     v
domain and domain-owned contracts
     ^
     |
infrastructure implementations
```

The configured layer mapping is:

| Conceptual layer | Direct directories below `backend/app` |
| --- | --- |
| API | `api` |
| Application | `application`, `services` |
| Domain | `domain` |
| Infrastructure | `db`, `infrastructure` |
| Explicit support | `config` |

Every scanned Python module below a direct `backend/app` subdirectory must belong to one configured layer or to the narrow
`support_directories` allowlist. Root modules are not implicitly composition roots: only the exact `composition_files`
allowlist (`__init__.py` and `main.py` in the baseline) receives that role. An undeclared directory such as `legacy`, or an
undeclared root module such as `orders.py`, produces `AR001` instead of becoming an unclassified route around the API and
persistence boundaries. Add a support directory or composition file only for a stable technical concern that does not
represent a hidden business layer.

The AST-based import check resolves ordinary, relative, lazy, `TYPE_CHECKING`, and guarded imports. Literal
`importlib.import_module` and `__import__` calls are recognized through statically provable imports and direct callable aliases;
relative `import_module` names are resolved only from a literal package or `package=__package__`. Lexical parameters and local
bindings shadow outer aliases, so a same-named application object is not mistaken for the standard library module. No import
target is executed during analysis. Canonical imports using either `app` or the configured multi-part source root such as
`backend.app` resolve to the same layer. The checker reports `AR001` for these exact forbidden pairs:

| Importing layer | Must not import |
| --- | --- |
| API | Infrastructure |
| Application | API, Infrastructure |
| Domain | API, Application, Infrastructure |
| Infrastructure | API, Application |

Application services may depend on domain contracts. Infrastructure may implement domain-owned contracts. Composition roots wire concrete infrastructure to consumers; routers must not reach directly into database implementations.

Scanned backend modules must not import frontend implementation modules. Cross-surface coordination belongs in framework-neutral contracts or external orchestration rather than a backend-to-frontend source dependency.

`AR002` additionally rejects direct domain imports from `alembic`, `fastapi`, `psycopg`, `pydantic`, `pydantic_settings`, `sqlalchemy`, and `starlette`. Domain behavior must not depend on HTTP, frontend, Tauri, or a concrete SQL provider.

FastAPI routers accept and validate requests, build DTOs, call an application service, and create responses. `AR004` rejects configured direct SQL/database imports and route handlers above 50 code lines. The checker deliberately avoids guessing that every calculation or external call is business logic; reviewers must still identify orchestration that belongs in a service.

### Frontend dependency direction

The intended direction is:

```text
App
 |
 v
features ------> api
 |
 v
shared
```

The TypeScript compiler AST is used for frontend import analysis rather than a source-text regular expression. The repository resolver then handles relative imports, the supported `@/`, `~/`, and `src/` root forms, source extensions, and directory indexes consistently. `AR001` prevents shared code from depending on API, UI, concrete feature modules, or the application entry point. It also prevents API/transport modules from depending on UI, feature implementation, or the application entry point. Feature modules must not import `main.ts`, `main.tsx`, or their JavaScript equivalents: the application entry point composes features and therefore remains above them. Shared modules are reusable leaves; API modules are transport adapters rather than UI owners.

When one feature consumes another, `AR003` requires the import to resolve to `features/<feature>/index.ts`, `index.tsx`, `index.js`, or `index.jsx`. Importing another feature's internal component, hook, store, or service is forbidden. Promote a stable public contract through the feature root or move genuinely shared behavior to `shared`.

### Tooling dependency direction

Quality-policy modules under `tools/quality` must remain independent of the public command orchestration in `tools/control.py` and `tools/control_parser.py`. General tooling modules must not import frontend implementation code. These `AR001` boundaries keep the checker reusable and prevent build or developer orchestration from coupling itself to a generated application surface.

### Tauri commands and Rust

Tauri commands are native-platform adapters. A command should validate or translate its input, call a focused native/application operation, and translate the result. Product behavior needed by the web client or backend must not be buried in a Tauri command.

The gate automates `cargo fmt --check`, Clippy with warnings denied, and `cargo check --locked`; Clippy and Cargo checking cover `--all-targets --all-features`. The Tauri test suite likewise runs locked Cargo tests across all targets and features. Repository-owned Rust analysis applies the central warning bands and hard limits with structured CQ findings, values, thresholds, and symbols. The function-line and parameter hard limits are also passed to Clippy as a defensive compiler-aware check. When a valid CQ101 or CQ104 exception covers a Rust symbol, the corresponding global Clippy threshold is widened only enough to prevent the duplicate lint from overriding that central exception; every non-excepted symbol is still enforced by the repository-owned finding. Clippy's cognitive-complexity lint is not used as a substitute for the policy's cyclomatic-complexity definition. **Whether a Tauri command remains a semantic adapter is a required review item in policy version 1.** The template does not yet have enough command/domain structure for a robust automated rule, and a keyword heuristic would create false confidence. Add an automated boundary only when real modules establish a stable dependency rule; until then, coding agents and reviewers must inspect command responsibility explicitly.

## Local use

Install the dependencies for the active profile, then run the complete gate from the repository root:

```sh
python tools/control.py install
python tools/control.py quality
```

Bare `quality` is identical to `quality check`. Focused actions support investigation without creating a second policy path:

| Command | Checks performed |
| --- | --- |
| `python tools/control.py quality` | Complete size, complexity, architecture, lint, type/compiler, and formatting gate |
| `python tools/control.py quality check` | Same complete gate as the bare command |
| `python tools/control.py quality size` | File/physical LOC plus supported function and class size checks |
| `python tools/control.py quality complexity` | Supported function complexity, nesting, parameter count, and frontend function-size metrics |
| `python tools/control.py quality architecture` | Backend, frontend, tooling dependency, and router rules |
| `python tools/control.py quality lint` | Ruff lint, ESLint, TypeScript compiler, Clippy, and Cargo check for enabled source areas |
| `python tools/control.py quality format` | Ruff format check, Prettier check, and rustfmt check for enabled source areas |
| `python tools/control.py quality --release` | Complete gate plus a blocking policy check for unsuppressed strong warnings |

Use machine-readable output for automation or agent processing:

```sh
python tools/control.py quality check --format json
```

Use an alternate configuration only for an intentional policy/test scenario:

```sh
python tools/control.py quality size --config path/to/code-quality.toml
```

Focused actions are diagnostic conveniences. Before hand-off, run the bare complete gate and the relevant tests.

## Diagnostics and reporting

Every repository-owned finding includes its severity, file, stable ID and name, measured value, applicable threshold, and required action. Symbol and line information are included when available. A representative hard-limit diagnostic is:

```text
CODE QUALITY ERROR

File: backend/app/services/application.py
Rule: CQ001 FILE_CODE_LINES
Actual: 901
Maximum: 900
Detail: File contains 901 code lines.
Required action: Split the file into smaller modules with clear responsibilities.
```

A non-blocking early warning follows the same model:

```text
CODE QUALITY WARNING

File: backend/app/services/project.py
Rule: CQ001 FILE_CODE_LINES
Actual: 684
Threshold: 600
Detail: File contains 684 code lines.
Recommendation: Split the file into smaller modules with clear responsibilities.
```

The final summary reports files checked, finding counts by severity, suppressed findings, and the status of size, complexity, lint, formatting, and architecture checks. Tool output remains attached to a failed tool check. JSON output contains the same rule IDs and structured finding data.

## CI behavior

Core CI has a dedicated `Core / Code Quality & Architecture` job. It installs the Python, frontend, and Rust quality dependencies, runs `python tools/control.py quality`, and verifies that checks did not rewrite tracked files. The core tooling, documentation, backend, frontend/build, and container jobs depend on this job, so the order is:

```text
push or pull request
        |
        v
quality gate
        |
        v
tests and builds
```

Profile-matrix CI runs the same quality gate inside every generated project after dependency installation and before its doctor, tests, and build. Generated PostgreSQL profiles do the same. Release Validation runs `python tools/control.py quality --release` before release-independent tests and artifact builds.

`WARNING` and `STRONG_WARNING` findings do not fail ordinary development and profile jobs. Release Validation blocks an unsuppressed `STRONG_WARNING`; every job blocks an unsuppressed `ERROR` or failed required tool check. Consequently, a handwritten file with exactly 900 code LOC leaves ordinary CI green but cannot enter a release without refactoring or a narrow expiring exception, while 901 code LOC produces an unsuppressible `CQ001 ERROR` everywhere. No workflow-specific ignore list changes that contract.

## Refactoring guidance

Treat a warning as time to understand the design, not as a request to split a file mechanically.

1. Identify the responsibilities, callers, side effects, and tests around the finding.
2. Choose a cohesive boundary with a name that expresses behavior, not an arbitrary line range.
3. Move behavior behind the smallest existing contract that fits; preserve public APIs where practical.
4. Keep dependency direction explicit. If infrastructure is leaking inward, introduce a domain-owned interface and wire its implementation at the composition root.
5. For a long function, prefer guard clauses and extracted testable operations. For high complexity, separate independent decisions instead of disguising them with comments.
6. For a long parameter list, group values only when they form a real request, configuration, DTO, or domain concept.
7. For an oversized class, split distinct reasons to change and keep collaboration explicit.
8. For router findings, move database access, calculations, filesystem work, and external-service orchestration into an application service.
9. Add or update focused tests before removing the old path, then run the complete quality gate.

Do not create one module for every small function, add empty Clean Architecture directories, weaken a threshold, or introduce a broad exception just to remove a finding.

## Rules for coding agents

`AGENTS.md` is part of the profile core and is copied into every generated project. Coding agents must follow these minimum rules:

1. Never create or extend a handwritten source file beyond 900 code lines.
2. Treat 900 LOC as a hard maximum, not a target.
3. Evaluate files above 600 LOC for splitting.
4. Normally refactor files above 750 LOC before adding significant functionality.
5. Never disable lint, type, formatting, architecture, test, or quality rules merely to complete a task.
6. Never add a quality exception without a specific documented architectural reason and expiry date.
7. Prefer existing modules and abstractions over duplicate utilities.
8. Keep routers, controllers, adapters, Tauri commands, and composition roots thin.
9. Put business logic in the appropriate application service or domain module.
10. Run `python tools/control.py quality` after structural code changes.
11. Run relevant tests after the quality gate.
12. Let existing repository architecture take precedence over implementation convenience.

Agents must report unresolved findings and test failures. They must not describe an excepted, skipped, or partial result as a clean complete run.

## Verification

Run the policy and focused documentation/profile regressions from the repository root:

```sh
python tools/control.py quality
python -m pytest tools/tests/test_docs_index.py tools/tests/test_profiles.py
```

For a structural cross-language change, follow with:

```sh
python tools/control.py test --suite all
```

Review JSON output when integrating another consumer:

```sh
python tools/control.py quality check --format json
```

## Risks and limitations

- Rust macro token trees are not expanded by the repository-owned analyzer. Written macro lines remain part of file/function LOC,
  and Cargo plus Clippy validate compiled expansions, but CQ scope metrics do not attribute generated functions or branches to a
  source symbol.
- The bundled Rust analyzer build removes inherited compiler wrappers, flags, target overrides, and release-profile overrides.
  Its versioned remapping contract canonicalizes the source root, user home, Cargo home and target, Rust sysroot, and dependency
  roots, with broad mappings before more specific mappings. The build rejects residual private physical paths in the artifact.
- Three relocated builds on the pinned Rust 1.97.1 Linux builder—registry dependencies, alternate homes and targets containing
  spaces, and in-tree vendored dependencies—produced identical bytes. Core CI and Release Validation rebuild and compare on
  Ubuntu; macOS and Windows Desktop jobs execute the checked-in artifact only. This is not evidence of cross-OS byte identity.
- Updating Syn, the JSON ABI, Wasmtime, a bound source input, the build contract, or the WASI artifact requires regenerating and
  re-verifying its provenance together; a missing, mismatched, corrupt, resource-exhausted, or ABI-incompatible artifact fails
  analysis instead of dropping metrics.
- The router rule detects robust structural signals but cannot prove the semantic absence of all business logic.
- Tauri adapter responsibility remains a semantic review item until stable internal module boundaries exist.
- Warnings require active review discipline because they intentionally do not block CI.

## Related documents

- [Framework architecture](architecture.md)
- [Project profiles](project-profiles.md)
- [Runtime configuration](configuration.md)
- [Continuous Integration](../tools/ci.md)
- [Tooling Guide](../tools/tooling.md)
- [Coding Agent Governance](../../AGENTS.md)

## Change log

| Date | Change | Author |
| --- | --- | --- |
| 2026-08-17 | Established code-quality and architecture governance version 1 | Project team |
