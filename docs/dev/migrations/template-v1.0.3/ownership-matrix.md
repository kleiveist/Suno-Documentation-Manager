<!-- AUTO-GENERATED:backlink START -->
[← Back](template-v1.0.3.md)
<!-- AUTO-GENERATED:backlink END -->
# Template v1.0.3 Ownership Matrix

| Field | Value |
| --- | --- |
| Status | APPROVED FOR TECHNICAL MIGRATION |
| Owner | SunoDM maintainers |
| Last review | 2026-08-25 |

## Decision model

`Migration class` selects the target content source. `Lifecycle ownership` is determined by membership in the reconstructed `desktop-local` baseline, except that runtime data and generated outputs are always protected or untracked. A managed integration file may intentionally differ from its baseline after product shaping.

| Path/area | Migration class | Lifecycle ownership | Current purpose | Template counterpart | Target source | Planned action | Risk | Verification |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `VERSION` | INTEGRATION | TEMPLATE_MANAGED | Product version source | Generated product version mirror | SunoDM | Keep `0.1.0`; never replace with template `1.0.3` | Bundle/version drift | Version check and manifest assertions |
| `project-profile.toml` | INTEGRATION | TEMPLATE_MANAGED | Active product profile | Generated profile | Matching product/scaffold state | Keep `desktop-local`, only `frontend` and `tauri` | Forbidden feature activation | Profile and config tests |
| `AGENTS.md` | TEMPLATE | TEMPLATE_MANAGED | Coding-agent governance | Released v1.0.3 policy | Reconstructed scaffold plus product data guard | Add policy; preserve product boundaries | New policy exposes inherited findings | Docs and quality checks |
| `.gitattributes` | TEMPLATE | TEMPLATE_MANAGED | Deterministic lifecycle/analyzer bytes | Released v1.0.3 file | Reconstructed scaffold | Add | Cross-platform byte drift | Diff and artifact hash |
| `.gitignore` | INTEGRATION | TEMPLATE_MANAGED | Product and generated-output exclusions | Released v1.0.3 file | Controlled merge | Add quality artifact rules; retain fpcalc exception | Source artifact accidentally ignored | `git check-ignore` and status scan |
| `LICENSE` | INTEGRATION | TEMPLATE_MANAGED | Product source license | Template uses MIT | Approved product decision and official standard text | Add the unmodified PolyForm Shield 1.0.0 text | Wrong or altered terms | Exact official-body hash review |
| `NOTICE` | INTEGRATION | PRODUCT_OWNED | Required product licensor notices | No baseline path | Approved product decision | Add the two downstream-required Blobbite.com notice lines | Wrong legal identity | Exact line/count review |
| `README.md` | INTEGRATION | TEMPLATE_MANAGED | Product contract and navigation | Generic scaffold README | SunoDM plus applicable lifecycle links | Preserve product README; add source-license/lifecycle facts | Product replaced by template demo claims | Docs check and full diff review |
| Community governance files | IGNORE | PRODUCT_OWNED | Absent at preflight | Master-only references | None without approved contacts | Keep absent | Invented contact or authority | Path and placeholder scan |
| `.github/workflows/**` | INTEGRATION | PRODUCT_OWNED | Product CI, absent at preflight | Master workflows are reference only | SunoDM commands and profile | Add validation workflows only; no publish/signing/Postgres | False CI claims or unauthorized release | Local YAML/tool checks; remote `NOT RUN` |
| `docs/dev/migrations/template-v1.0.3/**` | INTEGRATION | PRODUCT_OWNED | Migration evidence | No baseline path | Reviewed preflight and actual results | Add English reports; final report only after adoption | Premature PASS or local paths | Docs check and factual audit |
| Generic managed docs/navigation | INTEGRATION | TEMPLATE_MANAGED | Existing product-shaped references | v1.0.3 docs | Template conventions plus explicit product availability | Merge; regenerate navigation | Dead links or backend claims | PyGitIndex and docs check |
| Product docs and active ATPs | PRODUCT | PRODUCT_OWNED | Product contracts and history | None | SunoDM | Preserve; correct only current factual drift | Historical evidence corruption | Per-file diff and code/constants comparison |
| `docs/atp/completed/ATP-0001-template-lifecycle.md` | IGNORE | TEMPLATE_MANAGED | Absent; ID already product-owned | Baseline lifecycle ATP | None | Deliberately omit | Duplicate ATP identity | Exact missing-managed allowlist |
| Third-party notices/provenance | PRODUCT | PRODUCT_OWNED | Dependency and binary distribution terms | None | Existing upstream texts | Preserve bytes and applicability | Distribution noncompliance | Hash and notice inventory |
| `config/code-quality.toml` | TEMPLATE | TEMPLATE_MANAGED | Missing quality policy | Released v1.0.3 policy | Reconstructed scaffold | Add exact policy | Inherited product violations | Quality output, no hidden exception |
| `config/environment.toml` | INTEGRATION | TEMPLATE_MANAGED | Profile-filtered universal catalog | Matching baseline | Existing matching state | Keep; backend/database fields remain inactive | Catalog becomes runtime requirement | Config doctor and active-feature scan |
| `profiles/**`, `shared/**` | TEMPLATE/INTEGRATION | TEMPLATE_MANAGED | Feature/profile and shared tooling contracts | v1.0.3 baseline | Reconstructed baseline plus matching product identity | Preserve or merge exact baseline fixes | Profile/schema drift | Profile and schema tests |
| `tools/control.py`, `tools/control_parser.py` | INTEGRATION | TEMPLATE_MANAGED | Project CLI | v1.0.3 lifecycle/quality CLI | Template command map plus product behavior | Merge; add audit/adopt/status/verify; retain product commands | Removed command or activated backend | Help map and tooling tests |
| `tools/template_lifecycle/**` | TEMPLATE | TEMPLATE_MANAGED | Missing lifecycle system | v1.0.3 implementation | Reconstructed scaffold | Add exact implementation | Unsafe lifecycle mutation | Lifecycle test suite |
| `tools/quality/**` source/provenance | TEMPLATE | TEMPLATE_MANAGED | Missing quality engine | v1.0.3 implementation | Reconstructed scaffold | Add exact source/provenance | Policy/runtime mismatch | Quality and analyzer tests |
| Rust analyzer WASM | TEMPLATE | PRODUCT_OWNED | Missing pinned analyzer runtime | Generated artifact omitted by lifecycle manifests | Exact released artifact | Add byte-for-byte and document lifecycle gap | Untracked binary provenance | SHA-256/provenance/build check |
| `tools/config/**`, `tools/profiles/**`, `tools/inst/**`, `tools/tauri/**` | INTEGRATION | TEMPLATE_MANAGED | Existing product-shaped shared tooling | v1.0.3 baseline | Controlled merge | Take generic fixes; retain SunoDM names, sidecars, commands | Tool or bundle regression | Tools suite and dry runs |
| Universal backend/database tool modules | TEMPLATE | TEMPLATE_MANAGED | Present but inactive catalog support | v1.0.3 core | Baseline | Keep feature-gated; never activate for this profile | FastAPI/Postgres dependency | Profile-aware suite and command scan |
| `tools/tests/**` | INTEGRATION | TEMPLATE_MANAGED | Product tooling tests | v1.0.3 tests | Merge both | Add fixed lifecycle/quality/profile tests | Silent skip hides missing CI | Test inventory and skip review |
| `frontend/package.json` | INTEGRATION | TEMPLATE_MANAGED | Product dependencies/scripts | v1.0.3 quality scripts | Controlled merge | Keep private product/version/deps; add applicable scripts and license marker | Demo dependency or upgrade drift | Manifest/lock diff and npm tests |
| `frontend/package-lock.json` | INTEGRATION | TEMPLATE_MANAGED | Exact npm lock | Reconstructed lock | npm-generated merged manifest | Regenerate only if manifest changes | Uncontrolled upgrade | `npm ci`, lock review |
| Frontend quality/E2E configuration | TEMPLATE/INTEGRATION | TEMPLATE_MANAGED | Missing lint/format/coverage/e2e controls | v1.0.3 files | Reconstructed scaffold, product-shaped assertions | Add and adapt to SunoDM | Demo test gives false confidence | Frontend and E2E suites |
| `frontend/src/main.ts`, `index.html`, TS/Vite config | INTEGRATION | TEMPLATE_MANAGED | Product bootstrap/build shell | v1.0.3 starter | SunoDM behavior plus applicable scaffold configuration | Controlled merge; keep product root and no HTTP backend | Entry point/product loss | Typecheck, build, native smoke |
| `frontend/src/api/backend.ts` and test | IGNORE | TEMPLATE_MANAGED | Absent | FastAPI health adapter | None | Deliberately omit | Forbidden backend layer | Absence scan and exact missing allowlist |
| `frontend/src/app.ts`, domain, UI, demo | PRODUCT | PRODUCT_OWNED | Product UX and browser demo | None | SunoDM | Preserve behavior; no opportunistic format change | Large regression surface | Existing unit tests and manual smoke |
| `frontend/src/api/desktop.ts` | INTEGRATION | PRODUCT_OWNED | Typed native invoke adapter | None | SunoDM | Preserve all 54 mappings and progress/error behavior | IPC mismatch | Static invoke parity and frontend tests |
| `src-tauri/Cargo.toml` | INTEGRATION | TEMPLATE_MANAGED | Product Rust manifest | Minimal v1.0.3 manifest | Product deps/identity plus applicable metadata | Preserve name/version/MSRV/deps; add license marker only | Dependency or feature drift | Cargo metadata/check/test |
| `src-tauri/Cargo.lock` | INTEGRATION | TEMPLATE_MANAGED | Exact Rust dependency lock | Baseline lock | Cargo-generated product lock | Preserve unless manifest resolution requires regeneration | Supply-chain drift | Locked Cargo commands and diff |
| `src-tauri/tauri.conf.json` | INTEGRATION | TEMPLATE_MANAGED | Product identity, CSP, bundles, assets, sidecars | v1.0.3 config schema | Existing product plus applicable schema fixes | Preserve `sunodm`, `0.1.0`, identifier, resources, targets | Privilege/resource/version drift | Tauri doctor/build/config assertions |
| `src-tauri/capabilities/default.json` | INTEGRATION | TEMPLATE_MANAGED | Minimal `core:default` permission | v1.0.3 capability | Product minimum | Keep narrow capability | Privilege widening | Exact permission allowlist |
| `src-tauri/src/main.rs` | INTEGRATION | TEMPLATE_MANAGED | Native composition root | Minimal v1.0.3 entry | SunoDM | Keep modules, state, and all 54 registrations; merge only proven bootstrap fixes | Product command loss | Static parity and Rust tests |
| `src-tauri/src/lib.rs` | IGNORE | NOT_APPLICABLE | Single binary crate; absent | No baseline path | Existing architecture | Keep absent | Competing entry point | Cargo target inventory |
| `src-tauri/src/commands.rs` | INTEGRATION | PRODUCT_OWNED | Tauri transport/state | None | SunoDM | Preserve | Serialization/state regression | Command and parity tests |
| Rust application/domain/persistence modules | PRODUCT | PRODUCT_OWNED | Workspace, SQLite, tracks, evidence, hashes, certificates, documents | None | SunoDM | Preserve behavior and formats | Gate B/data loss risk | Rust suite and isolated fixture tests |
| Product fonts, sidecars, vendor code, testdata, fixtures | PRODUCT | PRODUCT_OWNED | Reproducibility, runtime assets, test evidence | None | Existing bytes | Preserve; add no generated product data | Supply-chain or golden drift | Hash/provenance and tests |
| `workflows/suno-track.toml` | PRODUCT | PRODUCT_OWNED | Authoritative workflow schema 1/version 1.9 | None | SunoDM | Preserve exact semantics | Product format drift | Parser and workflow tests |
| `.template/state.toml`, `.template/baseline.json` | GENERATED | TEMPLATE_MANAGED | Lifecycle provenance, not yet adopted | Reconstructed baseline | Adoption command only | Do not add during technical migration | Mixed technical/adoption commit | Gate D/E exact-path checks |
| Product workspaces, `.suno-doc/**`, SQLite/WAL/SHM, tracks | PRODUCT | PROTECTED_RUNTIME | User and runtime data | None | Runtime only | Never read, migrate, stage, or test in place | Data loss/disclosure | Path/extension/staged-file scan |
| Secrets and signing material | PRODUCT | PROTECTED_RUNTIME | Private runtime credentials | None | Runtime only | Never open or copy | Secret disclosure | Name-only and staged-file scan |
| Dependencies, caches, reports, build outputs, generated schemas | GENERATED | GENERATED_UNTRACKED | Local reproducible output | Ignored output | Tool generation | Never stage | Artifact pollution | Status and staged-file scan |

## Gate B boundary

Any semantic change to SQLite schema/migrations, persisted DTOs, workspace/track/evidence paths, hashes, certificates, manifests, timestamp compatibility, generated-document formats, or the 54-command IPC contract requires `FREIGABE DATEN- ODER FORMATÄNDERUNG`. No such change is planned.
