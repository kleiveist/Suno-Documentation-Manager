from __future__ import annotations

from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"

pytestmark = pytest.mark.skipif(
    not WORKFLOWS.exists(),
    reason="Master-repository CI workflows are not scaffolded into derived projects",
)


def _workflow(name: str) -> str:
    return (WORKFLOWS / name).read_text(encoding="utf-8")


def test_ci_workflows_have_safe_common_policy() -> None:
    for name in ("ci.yml", "profiles.yml", "postgres.yml", "desktop.yml"):
        content = _workflow(name)
        assert "pull_request:" in content
        assert "push:" in content
        assert "workflow_dispatch:" in content
        assert "- main" in content
        assert "permissions:\n  contents: read" in content
        assert "cancel-in-progress: ${{ github.event_name == 'pull_request' }}" in content
        assert "timeout-minutes:" in content
        assert "secrets." not in content
        assert "deploy" not in content.lower()
        assert "gh release" not in content.lower()
        assert "python tools/control.py release check" not in content


def test_core_ci_uses_supported_runtimes_and_public_tooling() -> None:
    content = _workflow("ci.yml")

    assert 'python-version: "3.11"' in content
    assert 'node-version: "20"' in content
    assert "python tools/control.py test --suite tools" in content
    assert "python tools/control.py test --suite schema" in content
    assert "python tools/control.py test --suite api" in content
    assert "python tools/control.py test --suite database" in content
    assert "python tools/control.py test --suite frontend" in content
    assert "python tools/control.py build web" in content
    assert "python tools/control.py container validate" in content
    assert "python tools/control.py build container" in content
    assert "python tools/control.py version check" in content
    assert "cache: pip" in content
    assert "cache: npm" in content
    assert "actions/checkout@v7" in content
    assert "actions/setup-python@v7" in content
    assert "actions/setup-node@v7" in content
    assert "\nenv:\n  DATABASE_URL:" not in content


def test_profile_matrix_generates_and_tests_every_profile() -> None:
    content = _workflow("profiles.yml")

    assert "fail-fast: false" in content
    for profile_id in (
        "web-only",
        "web-cloud",
        "desktop-local",
        "desktop-cloud",
        "full-platform",
    ):
        assert f"profile: {profile_id}" in content
    assert "python tools/control.py init" in content
    assert "python tools/control.py doctor" in content
    assert "python tools/control.py install --skip-playwright" in content
    assert "python tools/control.py test --suite all" in content
    assert "python tools/control.py build web" in content
    assert "python tools/control.py container validate" in content
    assert "python tools/control.py tauri doctor" in content
    assert "actions/checkout@v7" in content
    assert "actions/setup-python@v7" in content
    assert "actions/setup-node@v7" in content
    assert "actions/cache@v6" in content


def test_postgres_ci_uses_isolated_service_health_check_and_migration() -> None:
    content = _workflow("postgres.yml")

    assert "image: postgres:16" in content
    assert "POSTGRES_PASSWORD: test-password" in content
    assert "POSTGRES_DB: template_test" in content
    assert "DATABASE_URL_TEST:" in content
    assert "--health-cmd" in content
    assert "pg_isready" in content
    assert "sleep " not in content
    assert "python tools/control.py db upgrade" in content
    assert "python tools/control.py test --suite postgres" in content
    assert "--profile ${{ matrix.profile }}" in content
    assert "--with postgres" in content
    for profile_id in ("web-cloud", "desktop-cloud", "full-platform"):
        assert f"profile: {profile_id}" in content
    assert "python tools/control.py container validate" in content
    assert "actions/checkout@v7" in content
    assert "actions/setup-python@v7" in content
    assert "actions/setup-node@v7" in content


def test_desktop_ci_builds_unsigned_native_artifacts_on_each_platform() -> None:
    content = _workflow("desktop.yml")

    assert "workflow_call:" in content
    assert "ubuntu-latest" in content
    assert "macos-latest" in content
    assert "windows-latest" in content
    assert "target: linux" in content
    assert "target: macos" in content
    assert "target: windows" in content
    assert "python tools/control.py test --suite tauri" in content
    assert "python tools/control.py build desktop" in content
    assert "actions/upload-artifact@v6" in content
    assert "unsigned" in content.lower()
    assert "secrets." not in content
    assert "publish" not in content.lower()
    assert "deploy" not in content.lower()


def test_release_validation_is_explicit_and_never_publishes() -> None:
    content = _workflow("release.yml")

    assert "workflow_dispatch:" in content
    assert '"v*.*.*"' in content
    assert "branches:" not in content
    assert "python tools/control.py release check" in content
    assert "python tools/control.py build web" in content
    assert "python tools/control.py build container" in content
    assert "uses: ./.github/workflows/desktop.yml" in content
    assert "permissions:\n  contents: read" in content
    assert "secrets." not in content
    assert "publish" not in content.lower()
    assert "deploy" not in content.lower()
