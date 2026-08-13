from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

from tools.inst import docs_index


def _args(script: Path, **overrides) -> argparse.Namespace:
    values = {
        "script": str(script),
        "docs_dir": "docs",
        "dry_run": False,
        "force": False,
        "compact": False,
        "no_backlinks": False,
        "no_readme": False,
    }
    values.update(overrides)
    return argparse.Namespace(**values)


def test_generated_empty_labels_are_translated_but_authored_text_is_preserved(tmp_path) -> None:
    root = tmp_path / "project"
    docs = root / "docs"
    docs.mkdir(parents=True)
    readme = root / "README.md"
    readme.write_text(
        "<!-- AUTO-GENERATED:docs-index START -->\n"
        "- ⏭️ (keine Markdown-Dateien im Projekt-Root)\n"
        "<!-- AUTO-GENERATED:docs-index END -->\n"
        "Authored phrase: keine Seiten\n",
        encoding="utf-8",
    )
    index = docs / "index.md"
    index.write_text(
        "<!-- AUTO-GENERATED:docs-index START -->\n"
        "- ⏭️ (keine Seiten)\n"
        "<!-- AUTO-GENERATED:docs-index END -->\n",
        encoding="utf-8",
    )

    assert docs_index.normalize_generated_english(root) == 2
    assert "(no Markdown files in the project root)" in readme.read_text(encoding="utf-8")
    assert "Authored phrase: keine Seiten" in readme.read_text(encoding="utf-8")
    assert "(no pages)" in index.read_text(encoding="utf-8")


def test_index_command_uses_explicit_pygitindex_and_normalizes_output(monkeypatch, tmp_path) -> None:
    root = tmp_path / "project"
    docs = root / "docs"
    docs.mkdir(parents=True)
    script = tmp_path / "PyGitIndex.py"
    script.write_text("# fake", encoding="utf-8")
    (root / "README.md").write_text("# Project\n", encoding="utf-8")
    calls: list[tuple[list[str], Path]] = []

    def fake_run(command: list[str], cwd: Path, check: bool) -> subprocess.CompletedProcess[str]:
        calls.append((command, cwd))
        (docs / "index.md").write_text(
            "<!-- AUTO-GENERATED:docs-index START -->\n"
            "- ⏭️ (keine Seiten)\n"
            "<!-- AUTO-GENERATED:docs-index END -->\n",
            encoding="utf-8",
        )
        return subprocess.CompletedProcess(command, 0)

    monkeypatch.setattr(docs_index, "ROOT", root)
    monkeypatch.setattr(docs_index.subprocess, "run", fake_run)

    assert docs_index.main(_args(script)) == 0
    assert calls[0][1] == root
    assert calls[0][0][1] == str(script.resolve())
    assert "(no pages)" in (docs / "index.md").read_text(encoding="utf-8")


def test_index_dry_run_does_not_normalize_files(monkeypatch, tmp_path) -> None:
    root = tmp_path / "project"
    (root / "docs").mkdir(parents=True)
    script = tmp_path / "PyGitIndex.py"
    script.write_text("# fake", encoding="utf-8")
    monkeypatch.setattr(docs_index, "ROOT", root)
    monkeypatch.setattr(
        docs_index.subprocess,
        "run",
        lambda command, cwd, check: subprocess.CompletedProcess(command, 0),
    )
    normalized: list[Path] = []
    monkeypatch.setattr(docs_index, "normalize_generated_english", lambda path: normalized.append(path) or 0)

    assert docs_index.main(_args(script, dry_run=True)) == 0
    assert normalized == []
