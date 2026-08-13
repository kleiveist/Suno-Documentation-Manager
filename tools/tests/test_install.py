from __future__ import annotations

import subprocess
from pathlib import Path

from tools.inst import install


def test_frontend_install_uses_npm_ci_when_lockfile_exists(monkeypatch, tmp_path: Path) -> None:
    frontend = tmp_path / "frontend"
    frontend.mkdir()
    (frontend / "package.json").write_text("{}\n", encoding="utf-8")
    (frontend / "package-lock.json").write_text("{}\n", encoding="utf-8")
    calls: list[tuple[list[str], Path | None]] = []

    monkeypatch.setattr(install, "ROOT", tmp_path)
    monkeypatch.setattr(install.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(
        install,
        "_run",
        lambda command, cwd=None: calls.append((command, cwd))
        or subprocess.CompletedProcess(command, 0, stdout="", stderr=""),
    )

    result = install._install_frontend()

    assert result.status == "OK"
    assert calls == [(["/usr/bin/npm", "ci", "--no-audit", "--no-fund"], frontend)]


def test_frontend_install_falls_back_without_lockfile(monkeypatch, tmp_path: Path) -> None:
    frontend = tmp_path / "frontend"
    frontend.mkdir()
    (frontend / "package.json").write_text("{}\n", encoding="utf-8")
    calls: list[list[str]] = []

    monkeypatch.setattr(install, "ROOT", tmp_path)
    monkeypatch.setattr(install.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(
        install,
        "_run",
        lambda command, cwd=None: calls.append(command)
        or subprocess.CompletedProcess(command, 0, stdout="", stderr=""),
    )

    assert install._install_frontend().status == "OK"
    assert calls[0][1] == "install"


def test_backend_install_rebuilds_venv_when_python_is_missing(monkeypatch, tmp_path: Path) -> None:
    backend = tmp_path / "backend"
    venv = backend / ".venv"
    venv.mkdir(parents=True)
    python = venv / "bin" / "python"
    rebuild_reasons: list[str] = []

    def fake_rebuild(_venv_dir: Path, reason: str) -> tuple[bool, str]:
        rebuild_reasons.append(reason)
        python.parent.mkdir(parents=True, exist_ok=True)
        python.touch()
        return True, "venv rebuilt"

    monkeypatch.setattr(install, "ROOT", tmp_path)
    monkeypatch.setattr(install, "_venv_python", lambda _venv_dir: python)
    monkeypatch.setattr(install, "_rebuild_backend_venv", fake_rebuild)
    monkeypatch.setattr(install, "_ensure_backend_venv_consistency", lambda _python, _venv: (True, "ready"))
    monkeypatch.setattr(
        install,
        "_run",
        lambda command, cwd=None: subprocess.CompletedProcess(command, 0, stdout="", stderr=""),
    )

    ok, message = install._install_backend_with_pip(backend, [])

    assert ok is True
    assert message == "pip/venv backend install completed"
    assert rebuild_reasons == [f"venv python is missing at {python}"]


def test_tooling_install_rebuilds_inconsistent_existing_venv(monkeypatch, tmp_path: Path) -> None:
    tools_dir = tmp_path / "tools"
    tooling_venv = tools_dir / ".venv"
    tooling_python = tooling_venv / "bin" / "python"
    tooling_venv.mkdir(parents=True)
    requirements = tools_dir / "requirements.txt"
    requirements.write_text("pytest==8.2.2\n", encoding="utf-8")
    rebuilds: list[tuple[Path, bool]] = []

    def fake_create(venv_dir: Path, clear: bool) -> tuple[bool, str]:
        rebuilds.append((venv_dir, clear))
        tooling_python.parent.mkdir(parents=True, exist_ok=True)
        tooling_python.touch()
        return True, "venv rebuilt"

    monkeypatch.setattr(install, "ROOT", tmp_path)
    monkeypatch.setattr(install, "TOOLS_VENV", tooling_venv)
    monkeypatch.setattr(install, "TOOLS_REQUIREMENTS", requirements)
    monkeypatch.setattr(install, "_runtime_imports", lambda _python, _modules: False)
    monkeypatch.setattr(install, "_inspect_backend_venv", lambda _python, _venv: (False, "interpreter mismatch"))
    monkeypatch.setattr(install, "_create_backend_venv", fake_create)
    monkeypatch.setattr(
        install,
        "_run",
        lambda command, cwd=None: subprocess.CompletedProcess(command, 0, stdout="", stderr=""),
    )

    result = install._install_tooling_runtime()

    assert result.status == "OK"
    assert rebuilds == [(tooling_venv, True)]
