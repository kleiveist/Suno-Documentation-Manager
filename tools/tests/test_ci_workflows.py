from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
DEPENDABOT = ROOT / ".github" / "dependabot.yml"
EXPECTED_WORKFLOWS = {"ci.yml", "desktop.yml", "release.yml", "security.yml"}
ACTION_PINS = {
    "actions/cache": "55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
    "actions/checkout": "3d3c42e5aac5ba805825da76410c181273ba90b1",
    "actions/setup-node": "820762786026740c76f36085b0efc47a31fe5020",
    "actions/setup-python": "5fda3b95a4ea91299a34e894583c3862153e4b97",
    "actions/upload-artifact": "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
    "anchore/sbom-action": "e22c389904149dbc22b58101806040fa8d37a610",
    "github/codeql-action/analyze": "db488ddef3bf6cb639b32c2e9a7c0a7ea8271d28",
    "github/codeql-action/init": "db488ddef3bf6cb639b32c2e9a7c0a7ea8271d28",
}


def _workflow(name: str) -> str:
    return (WORKFLOWS / name).read_text(encoding="utf-8")


def _jobs(content: str) -> set[str]:
    jobs = content.split("\njobs:\n", maxsplit=1)[1]
    return set(re.findall(r"^  ([a-z][a-z0-9-]*):$", jobs, flags=re.MULTILINE))


def _assert_exact_action_pins(content: str) -> None:
    remote_uses = [
        line.strip()
        for line in content.splitlines()
        if line.strip().startswith("uses: ") and not line.strip().startswith("uses: ./")
    ]
    matches = re.findall(r"^\s*uses: ([^@\s]+)@([0-9a-f]{40})(?:\s+#.*)?$", content, flags=re.MULTILINE)

    assert len(matches) == len(remote_uses)
    for action, pin in matches:
        assert action in ACTION_PINS
        assert pin == ACTION_PINS[action]


def test_product_owns_only_applicable_workflows() -> None:
    assert {path.name for path in WORKFLOWS.glob("*.yml")} == EXPECTED_WORKFLOWS
    for name in ("profiles.yml", "postgres.yml", "release-publish.yml"):
        assert not (WORKFLOWS / name).exists()


def test_workflows_use_exact_v1_0_3_action_pins_and_no_external_authority() -> None:
    for name in sorted(EXPECTED_WORKFLOWS):
        content = _workflow(name)
        _assert_exact_action_pins(content)
        assert "secrets." not in content
        assert "contents: write" not in content
        assert "gh release" not in content.lower()
        assert "git tag" not in content.lower()
        assert "tauri signer" not in content.lower()
        assert "cosign" not in content.lower()
        assert "actions/attest@" not in content
        assert "deploy" not in content.lower()
        assert "publish" not in content.lower()


def test_continuous_workflows_have_safe_common_policy() -> None:
    for name in ("ci.yml", "desktop.yml", "security.yml"):
        content = _workflow(name)
        assert "pull_request:" in content
        assert "push:" in content
        assert "workflow_dispatch:" in content
        assert "- main" in content
        assert "permissions:\n  contents: read" in content
        assert "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" in content
        assert "timeout-minutes:" in content


def test_dependabot_covers_only_product_dependency_ecosystems() -> None:
    content = DEPENDABOT.read_text(encoding="utf-8")

    assert content.startswith("version: 2\n")
    assert content.count("package-ecosystem: github-actions") == 1
    assert content.count("package-ecosystem: npm") == 1
    assert content.count("package-ecosystem: pip") == 1
    assert content.count("package-ecosystem: cargo") == 2
    for directory in (
        'directory: "/"',
        'directory: "/frontend"',
        'directory: "/tools"',
        'directory: "/src-tauri"',
        'directory: "/tools/quality/rust_analyzer"',
    ):
        assert content.count(directory) == 1
    for excluded in ('directory: "/backend"', 'directory: "/deployment"', "package-ecosystem: docker"):
        assert excluded not in content
    assert content.count("interval: weekly") == 5
    assert content.count("open-pull-requests-limit: 3") == 5
    assert content.count("groups:") == 5
    assert content.count("applies-to: version-updates") == 5
    assert content.count("semver-major-days: 30") == 4
    assert "          - major" not in content


def test_core_ci_matches_the_desktop_local_product_profile() -> None:
    content = _workflow("ci.yml")

    assert _jobs(content) == {"quality", "tooling", "documentation", "frontend", "e2e", "msrv"}
    assert content.count("needs: quality") == 5
    assert 'python-version: "3.11"' in content
    assert 'node-version: "24"' in content
    assert "python tools/control.py quality" in content
    assert "python tools/control.py test --suite tools" in content
    assert "python tools/control.py docs check" in content
    assert "python tools/control.py test --suite frontend" in content
    assert "python tools/control.py build web" in content
    assert "python tools/control.py test --suite e2e" in content
    assert "cargo check --manifest-path src-tauri/Cargo.toml --locked --all-targets" in content
    for excluded in (
        "\n  backend:",
        "\n  container:",
        "DATABASE_URL",
        "backend/requirements",
        "python tools/control.py test --suite api",
        "python tools/control.py test --suite database",
        "python tools/control.py test --suite postgres",
        "python tools/control.py container",
        "python tools/control.py build container",
    ):
        assert excluded not in content


def test_quality_job_verifies_the_pinned_analyzer_and_real_ast_path() -> None:
    content = _workflow("ci.yml")

    assert (
        "rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy --target wasm32-wasip1" in content
    )
    assert "rustup default 1.97.1" in content
    assert "tools/quality/rust_analyzer/target" in content
    assert "tools/quality/rust_analyzer/Cargo.lock" in content
    assert "python tools/quality/rust_analyzer/build.py --check" in content
    assert "cargo clippy --manifest-path tools/quality/rust_analyzer/Cargo.toml" in content
    assert "tools/tests/quality/test_rust_syntax.py" in content
    assert "tools/tests/quality/test_rust_payload.py" in content
    assert "tools/tests/quality/test_rust_wasi_host.py" in content
    assert "python tools/control.py install --skip-backend --skip-playwright" in content
    assert "tools/.venv/bin/python -m pytest -q tools/tests/quality/test_typescript_ast.py" in content


def test_e2e_job_starts_only_the_local_frontend_and_always_stops_it() -> None:
    content = _workflow("ci.yml")
    start = "python tools/control.py run --detach"
    browser_test = "python tools/control.py test --suite e2e"
    stop = "python tools/control.py stop"

    assert "npx playwright install --with-deps chromium" in content
    assert content.index(start) < content.index(browser_test) < content.index(stop)
    assert (
        "      - name: Stop the local frontend\n        if: always()\n        run: python tools/control.py stop\n"
    ) in content
    assert "python tools/control.py install --skip-backend --skip-tooling --skip-playwright" in content


def test_ci_checks_the_declared_rust_msrv_without_weakening_the_quality_toolchain() -> None:
    content = _workflow("ci.yml")

    assert "name: Core / Rust 1.88 MSRV" in content
    assert "rustup toolchain install 1.88.0 --profile minimal" in content
    assert "rustup default 1.88.0" in content
    assert "${{ runner.os }}-cargo-msrv-1.88-" in content
    assert "cargo check --manifest-path src-tauri/Cargo.toml --locked --all-targets" in content
    assert "rustup toolchain install 1.97.1" in content


def test_desktop_ci_builds_unsigned_sunodm_artifacts_on_all_platforms() -> None:
    content = _workflow("desktop.yml")

    assert "workflow_call:" in content
    for os_name in ("ubuntu-latest", "macos-latest", "windows-latest"):
        assert os_name in content
    for target in ("linux", "macos", "windows"):
        assert f"target: {target}" in content
        assert f"sunodm-desktop-{target}-unsigned" in content
    assert "rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy" in content
    assert "python tools/control.py doctor" in content
    assert "python tools/control.py test --suite tauri" in content
    assert "python tools/control.py build desktop" in content
    assert "python tools/control.py tauri verify-artifacts" in content
    assert "name: sunodm-desktop-${{ matrix.target }}-unsigned" in content
    assert "DATABASE_URL" not in content
    assert "unsigned" in content.lower()


def test_desktop_ci_preserves_linux_bundle_and_archive_safety_contracts() -> None:
    content = _workflow("desktop.yml")
    expected_input = (
        "      linux_bundles:\n"
        "        description: Comma-separated unsigned Linux bundle targets\n"
        "        required: false\n"
        "        default: deb\n"
        "        type: string\n"
    )

    assert content.count(expected_input) == 2
    assert "LINUX_BUNDLES: ${{ inputs.linux_bundles || matrix.bundles }}" in content
    assert '--bundles "$LINUX_BUNDLES"' in content
    assert '--summary-file "$GITHUB_STEP_SUMMARY"' in content
    assert 'tar --create --gzip --file "$archive_path" -- "${archive_inputs[@]}"' in content
    assert "[System.IO.FileAttributes]::ReparsePoint" in content
    assert "[System.IO.Compression.ZipFile]::OpenRead" in content
    assert "if-no-files-found: error" in content
    assert "include-hidden-files: true" in content
    assert "compression-level: 0" in content


def test_security_workflow_has_least_privilege_product_scanners() -> None:
    content = _workflow("security.yml")

    assert "if: github.event_name == 'pull_request'" in content
    assert 'python-version: "3.11"' in content
    assert 'node-version: "24"' in content
    assert "rustup toolchain install 1.97.1 --profile minimal" in content
    assert "python -m pip_audit --requirement tools/requirements.txt --strict --progress-spinner off" in content
    assert "npm audit --audit-level=high" in content
    assert "cargo install cargo-audit --locked --version 0.22.2 --no-default-features" in content
    assert "cargo audit" in content
    assert "--ignore-vuln" not in content
    assert "--upgrade pip" not in content
    assert "security-events: write" in content
    assert "fail-fast: false" in content
    for language in ("javascript-typescript", "python", "rust"):
        assert f"- {language}" in content
    assert "build-mode: none" in content
    assert "queries: security-extended" in content


def test_release_workflow_validates_sunodm_without_signing_or_publication() -> None:
    content = _workflow("release.yml")

    assert _jobs(content) == {"validate", "desktop-candidates"}
    assert "workflow_dispatch:" in content
    assert '"v*.*.*"' in content
    assert "branches:" not in content
    assert "permissions:\n  contents: read" in content
    assert "python tools/control.py quality --release" in content
    assert "python tools/control.py docs check" in content
    assert "python tools/control.py test --suite all" in content
    assert "python tools/control.py build web" in content
    assert "python tools/control.py release check" in content
    assert "python tools/control.py build container" not in content
    assert "actions/attest@" not in content
    assert "attestations: write" not in content
    assert "id-token: write" not in content
    assert "contents: write" not in content
    assert "secrets." not in content


def test_release_workflow_emits_sunodm_validation_evidence_and_unsigned_desktops() -> None:
    content = _workflow("release.yml")
    start = "python tools/control.py run --detach"
    full_test = "python tools/control.py test --suite all"
    stop = "python tools/control.py stop"

    assert "artifact-name: sunodm-${{ github.sha }}.spdx.json" in content
    assert "output-file: .dist/sbom/sunodm-${{ github.sha }}.spdx.json" in content
    assert "upload-artifact: false" in content
    assert "upload-release-assets: false" in content
    assert "name: sunodm-web-release-validation" in content
    assert ".dist/web/*.zip" in content
    assert ".dist/sbom/*.spdx.json" in content
    assert content.index(start) < content.index(full_test) < content.index(stop)
    assert "uses: ./.github/workflows/desktop.yml" in content
    assert 'linux_bundles: "deb,rpm,appimage"' in content
