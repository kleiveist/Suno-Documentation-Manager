from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

from tools import logger

ROOT = Path(__file__).resolve().parents[2]
INDEX_START = "<!-- AUTO-GENERATED:docs-index START -->"
INDEX_END = "<!-- AUTO-GENERATED:docs-index END -->"
SCRIPT_ENV = "PYGITINDEX_PATH"

ENGLISH_REPLACEMENTS = {
    "(keine Seiten)": "(no pages)",
    "(keine Markdown-Dateien im Projekt-Root)": "(no Markdown files in the project root)",
}


def _script_candidates(explicit: str | None = None) -> list[Path]:
    candidates: list[Path] = []
    configured = explicit or os.environ.get(SCRIPT_ENV)
    if configured:
        candidates.append(Path(configured).expanduser())

    for command_name in ("PyGitIndex", "PyGitIndex.py", "pygitindex"):
        command = shutil.which(command_name)
        if command:
            candidates.append(Path(command))

    candidates.extend(
        [
            Path.home() / "Dokumente" / "Python" / "bin" / "PyGit" / "PyGitIndex.py",
            Path.home() / "Documents" / "Python" / "bin" / "PyGit" / "PyGitIndex.py",
        ]
    )
    return candidates


def find_script(explicit: str | None = None) -> Path | None:
    if explicit:
        candidate = Path(explicit).expanduser()
        return candidate.resolve() if candidate.is_file() else None
    for candidate in _script_candidates(explicit):
        if candidate.is_file():
            return candidate.resolve()
    return None


def _translate_generated_block(text: str) -> str:
    pattern = re.compile(re.escape(INDEX_START) + r".*?" + re.escape(INDEX_END), re.DOTALL)

    def translate(match: re.Match[str]) -> str:
        block = match.group(0)
        for source, target in ENGLISH_REPLACEMENTS.items():
            block = block.replace(source, target)
        return block

    return pattern.sub(translate, text)


def normalize_generated_english(project_root: Path = ROOT) -> int:
    candidates = [project_root / "README.md", *(project_root / "docs").rglob("*.md")]
    changed = 0
    for path in candidates:
        if not path.is_file():
            continue
        original = path.read_text(encoding="utf-8")
        updated = _translate_generated_block(original)
        if updated == original:
            continue
        path.write_text(updated, encoding="utf-8", newline="\n")
        changed += 1
    return changed


def _command_for(script: Path, args: argparse.Namespace) -> list[str]:
    command = [sys.executable, str(script), "--docs-dir", args.docs_dir]
    if args.dry_run:
        command.append("--dry-run")
    if args.force:
        command.append("--force")
    if args.compact:
        command.append("--readme-compact-docs")
    if args.no_backlinks:
        command.extend(["--no-docs-backlinks", "--no-root-backlinks"])
    if args.no_readme:
        command.append("--no-readme")
    return command


def main(args: argparse.Namespace) -> int:
    script = find_script(args.script)
    if script is None:
        logger.fail("PyGitIndex was not found.")
        logger.info(
            "Set PYGITINDEX_PATH or pass '--script /path/to/PyGitIndex.py', then run this command again."
        )
        return 1

    command = _command_for(script, args)
    logger.info(f"Using PyGitIndex: {script}")
    logger.info(f"Project root: {ROOT}")
    sys.stdout.flush()
    completed = subprocess.run(command, cwd=ROOT, check=False)
    if completed.returncode != 0:
        logger.fail(f"PyGitIndex failed with exit code {completed.returncode}")
        return completed.returncode

    if args.dry_run:
        logger.ok("Documentation index preview completed; no project files were changed")
        return 0

    translated = normalize_generated_english(ROOT)
    if translated:
        logger.info(f"Normalized generated English labels in {translated} file(s)")
    logger.ok("Documentation indices and backlinks are up to date")
    return 0
