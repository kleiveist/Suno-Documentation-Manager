from __future__ import annotations

from tools import control


def test_bare_control_prints_root_help(capsys) -> None:
    assert control.main([]) == 0

    output = capsys.readouterr().out
    assert "Recommended workflow after returning to the project" in output
    assert "command map" in output
    assert "console" in output
    assert "docs" in output


def test_bare_build_prints_target_map_without_building(capsys) -> None:
    assert control.main(["build"]) == 0

    output = capsys.readouterr().out
    assert "build targets" in output
    assert "web" in output
    assert "desktop" in output


def test_bare_test_prints_suite_map_without_running(capsys) -> None:
    assert control.main(["test"]) == 0

    output = capsys.readouterr().out
    assert "Test map" in output
    assert "api" in output
    assert "tools" in output


def test_bare_docs_prints_documentation_map(capsys) -> None:
    assert control.main(["docs"]) == 0

    output = capsys.readouterr().out
    assert "documentation actions" in output
    assert "PyGitIndex" in output
    assert "index" in output


def test_legacy_desktop_build_alias_is_normalized() -> None:
    assert control._normalize_argv(["--build", "--desktop", "--dry-run"]) == [
        "build",
        "desktop",
        "--dry-run",
    ]
