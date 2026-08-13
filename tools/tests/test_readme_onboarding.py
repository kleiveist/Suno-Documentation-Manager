from __future__ import annotations

import re
from pathlib import Path

from tools import control
from tools.profiles import runtime as profile_runtime


ROOT = Path(__file__).resolve().parents[2]
README = ROOT / "README.md"


def _readme() -> str:
    return README.read_text(encoding="utf-8")


def test_readme_is_product_onboarding() -> None:
    content = _readme()

    assert content.startswith("# Suno Documentation Manager\n")
    assert content.index("## Development quick start") < content.index("## Detailed documentation")
    assert "python tools/control.py tauri run --foreground" in content
    assert "Track Documentation Completion Certificate" in content


def test_quickstart_commands_and_examples_match_the_cli_parser() -> None:
    parser = control._build_parser()
    examples = [
        ["doctor"],
        ["install"],
        ["test", "--suite", "all"],
        ["build", "web"],
        ["build", "desktop"],
        ["docs", "index", "--dry-run"],
        ["release", "check"],
        ["tauri", "doctor"],
        ["tauri", "run", "--foreground"],
    ]

    for example in examples:
        parser.parse_args(example)


def test_readme_matches_the_active_local_desktop_profile() -> None:
    profile = profile_runtime.active_profile(ROOT)
    content = _readme()

    assert profile.profile_id == "desktop-local"
    assert profile.features == ("frontend", "tauri")
    assert "There is no product backend and no network dependency." in content
    assert "src-tauri/" in content
    assert "backend/" not in content
    assert "deployment/" not in content


def test_readme_does_not_document_a_product_server_endpoint() -> None:
    content = _readme()

    assert "http://127.0.0.1:8000" not in content
    assert "/api/health" not in content
    assert "/api/ready" not in content


def test_all_local_readme_links_resolve() -> None:
    targets = re.findall(r"\[[^\]]+\]\(([^)]+)\)", _readme())

    for target in targets:
        if target.startswith(("http://", "https://", "#")):
            continue
        relative_path = target.split("#", 1)[0]
        assert (ROOT / relative_path).exists(), f"README link does not exist: {target}"
