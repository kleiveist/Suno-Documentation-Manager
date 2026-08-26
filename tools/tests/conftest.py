from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from tools.profiles import cli as profile_cli

ROOT = Path(__file__).resolve().parents[2]
IS_MANAGED_PRODUCT = (ROOT / ".template" / "state.toml").is_file()


def _is_template_source() -> bool:
    completed = subprocess.run(
        ["git", "config", "--get", "remote.origin.url"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    remote = completed.stdout.strip().lower().removesuffix(".git").rstrip("/")
    return completed.returncode == 0 and remote.endswith("/kleiveist/template-projekte")


IS_TEMPLATE_SOURCE = _is_template_source()


@pytest.fixture(autouse=True)
def isolate_nested_generator_tests_from_lifecycle_finalization(monkeypatch: pytest.MonkeyPatch) -> None:
    """Keep scaffold-only tests applicable after the suite is copied to a product.

    A managed product is not a canonical template Git source, so nested generator
    tests exercise their historical scaffold assertions without inventing false
    provenance. Lifecycle finalization itself has dedicated master-checkout tests.
    """

    if IS_MANAGED_PRODUCT or not IS_TEMPLATE_SOURCE:
        monkeypatch.setattr(profile_cli, "finalize_generated_project", lambda _plan: None)
