from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FRONTEND = ROOT / "frontend"


def test_frontend_exposes_stable_quality_scripts() -> None:
    package = json.loads((FRONTEND / "package.json").read_text(encoding="utf-8"))

    assert package["scripts"]["typecheck"] == "tsc --noEmit"
    assert package["scripts"]["lint"] == "eslint ."
    assert package["scripts"]["format:check"] == "prettier --check ."
    assert package["scripts"]["build"] == "npm run typecheck && vite build && npm run performance:check"
    assert package["scripts"]["test"] == "npm run test:coverage"
    assert package["scripts"]["test:coverage"] == "vitest run --coverage"
    assert package["scripts"]["test:e2e"] == "playwright test"
    assert package["scripts"]["performance:check"] == "node scripts/check-bundle-budget.mjs"

    dependencies = package["devDependencies"]
    for dependency in (
        "@eslint/js",
        "@axe-core/playwright",
        "@playwright/test",
        "@types/node",
        "@vitest/coverage-v8",
        "eslint",
        "eslint-config-prettier",
        "globals",
        "prettier",
        "typescript-eslint",
    ):
        assert dependency in dependencies
    assert dependencies["eslint"].startswith("^9.")
    assert dependencies["@eslint/js"].startswith("^9.")
    assert dependencies["prettier"].startswith("^3.")
    assert dependencies["vite"].startswith("^8.")
    assert dependencies["vitest"].startswith("^4.")
    assert dependencies["@vitest/coverage-v8"].startswith("^4.")
    assert dependencies["@types/node"].startswith("^24.")


def test_frontend_test_baseline_has_enforced_coverage_accessibility_and_performance() -> None:
    index = (FRONTEND / "index.html").read_text(encoding="utf-8")
    vitest = (FRONTEND / "vitest.config.ts").read_text(encoding="utf-8")
    playwright = (FRONTEND / "playwright.config.ts").read_text(encoding="utf-8")
    e2e = (FRONTEND / "tests" / "e2e" / "starter.spec.ts").read_text(encoding="utf-8")
    budget = json.loads((FRONTEND / "bundle-budget.json").read_text(encoding="utf-8"))

    assert 'provider: "v8"' in vitest
    assert "all: true" not in vitest
    assert "perFile: true" in vitest
    assert "branches: 50" in vitest
    assert "functions: 58" in vitest
    assert "lines: 61" in vitest
    assert "statements: 57" in vitest
    assert 'testDir: "./tests/e2e"' in playwright
    assert 'outputDir: "coverage/e2e-results"' in playwright
    assert 'trace: "retain-on-failure"' in playwright
    assert 'from "@axe-core/playwright"' in e2e
    assert '<html lang="en">' in index
    assert 'toHaveAttribute("lang", "en")' in e2e
    assert 'toHaveTitle("Suno Documentation Manager")' in e2e
    assert 'name: "Choose workspace"' in e2e
    assert 'getByText("No cloud, no login, no telemetry.")' in e2e
    assert "backend-status" not in e2e

    assert budget["schemaVersion"] == 1
    assert budget["limits"] == {
        "entryHtmlBytes": 16384,
        "javascriptBytes": 524288,
        "stylesheetBytes": 114688,
        "totalBytes": 638976,
    }

    tsconfig = json.loads((FRONTEND / "tsconfig.json").read_text(encoding="utf-8"))
    assert tsconfig["include"] == ["src", "tests", "*.config.ts"]
    assert tsconfig["compilerOptions"]["types"] == ["node"]


def test_eslint_uses_flat_typescript_and_prettier_configuration() -> None:
    config = (FRONTEND / "eslint.config.js").read_text(encoding="utf-8")

    assert 'from "@eslint/js"' in config
    assert 'from "typescript-eslint"' in config
    assert 'from "eslint-config-prettier/flat"' in config
    assert 'ignores: ["coverage/**", "dist/**"]' in config
    assert '"public/**/*.js"' in config

    # Governance thresholds come from config/code-quality.toml via the quality orchestrator.
    for duplicated_threshold_rule in (
        "complexity",
        "max-depth",
        "max-lines-per-function",
        "max-params",
    ):
        assert duplicated_threshold_rule not in config
