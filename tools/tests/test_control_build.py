from __future__ import annotations

import subprocess
import zipfile
from pathlib import Path

import pytest

from tools import control
from tools.inst import build


def test_build_alias_is_normalized() -> None:
    assert control._normalize_argv(["--build"]) == ["build", "web"]
    assert control._normalize_argv(["tauri", "--build", "--dry-run"]) == [
        "tauri",
        "build",
        "--dry-run",
    ]


def test_build_command_is_dispatched(monkeypatch) -> None:
    calls: list[str] = []

    def fake_build(args) -> int:
        calls.append(args.command)
        return 0

    monkeypatch.setattr(control.build, "main", fake_build)

    assert control.main(["--build"]) == 0
    assert calls == ["build"]


def test_web_build_runs_npm_build_and_reports_dist(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    dist = frontend / "dist"
    web_artifact_dir = root / ".dist" / "web"
    web_zip_path = web_artifact_dir / "template-project-web.zip"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")
    (root / "LICENSE").write_text("product license\n", encoding="utf-8")
    (root / "NOTICE").write_text("required notice\n", encoding="utf-8")
    (root / "THIRD_PARTY_NOTICES.md").write_text("third-party notices\n", encoding="utf-8")
    calls: list[tuple[list[str], Path | None]] = []

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        calls.append((cmd, cwd))
        (dist / "assets").mkdir(parents=True)
        (dist / "index.html").write_text("<html></html>", encoding="utf-8")
        (dist / "assets" / "app.js").write_text("console.log('ok');", encoding="utf-8")
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", dist)
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", web_artifact_dir)
    monkeypatch.setattr(build, "WEB_ZIP_PATH", web_zip_path)
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 0
    assert calls == [(["/usr/bin/npm", "run", "build"], frontend)]
    assert web_zip_path.exists()
    with zipfile.ZipFile(web_zip_path) as archive:
        assert sorted(archive.namelist()) == [
            "LICENSE",
            "NOTICE",
            "THIRD_PARTY_NOTICES.md",
            "assets/app.js",
            "index.html",
        ]
        assert archive.read("LICENSE") == b"product license\n"
        assert archive.read("NOTICE") == b"required notice\n"
        assert archive.read("THIRD_PARTY_NOTICES.md") == b"third-party notices\n"


def test_web_build_fails_when_distribution_notice_is_missing(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    dist = frontend / "dist"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")
    (root / "LICENSE").write_text("product license\n", encoding="utf-8")
    (root / "THIRD_PARTY_NOTICES.md").write_text("third-party notices\n", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        dist.mkdir()
        (dist / "index.html").write_text("<html></html>", encoding="utf-8")
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", dist)
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "sunodm-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1
    assert not (root / ".dist" / "web" / "sunodm-web.zip").exists()


def test_web_build_rejects_portable_legal_filename_collision(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    dist = frontend / "dist"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")
    for name in build.WEB_LEGAL_FILES:
        (root / name).write_text(f"{name}\n", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        dist.mkdir()
        (dist / "index.html").write_text("<html></html>", encoding="utf-8")
        (dist / "license").write_text("shadow\n", encoding="utf-8")
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", dist)
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "sunodm-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1
    assert not (root / ".dist" / "web" / "sunodm-web.zip").exists()


def test_web_build_rejects_symlinks(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    dist = frontend / "dist"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")
    for name in build.WEB_LEGAL_FILES:
        (root / name).write_text(f"{name}\n", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        dist.mkdir()
        (dist / "index.html").write_text("<html></html>", encoding="utf-8")
        try:
            (dist / "outside-link").symlink_to(root / "NOTICE")
        except OSError:
            pytest.skip("symbolic links are unavailable on this platform")
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", dist)
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "sunodm-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1
    assert not (root / ".dist" / "web" / "sunodm-web.zip").exists()


def test_web_build_fails_when_npm_is_missing(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", frontend / "dist")
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "template-project-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: None)

    assert build.main(control._build_parser().parse_args(["build"])) == 1


def test_web_build_fails_when_dist_is_not_created(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", frontend / "dist")
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "template-project-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1


def test_web_build_returns_failure_for_failed_npm_build(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(cmd, 1, stdout="type error", stderr="vite failed")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", frontend / "dist")
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "template-project-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1


def test_web_build_fails_when_dist_is_empty(monkeypatch, tmp_path) -> None:
    root = tmp_path / "repo"
    frontend = root / "frontend"
    dist = frontend / "dist"
    frontend.mkdir(parents=True)
    (frontend / "package.json").write_text("{}", encoding="utf-8")

    def fake_run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        dist.mkdir()
        return subprocess.CompletedProcess(cmd, 0, stdout="built", stderr="")

    monkeypatch.setattr(build, "ROOT", root)
    monkeypatch.setattr(build, "FRONTEND_DIR", frontend)
    monkeypatch.setattr(build, "DIST_DIR", dist)
    monkeypatch.setattr(build, "WEB_ARTIFACT_DIR", root / ".dist" / "web")
    monkeypatch.setattr(build, "WEB_ZIP_PATH", root / ".dist" / "web" / "template-project-web.zip")
    monkeypatch.setattr(build.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)
    monkeypatch.setattr(build, "_run", fake_run)

    assert build.main(control._build_parser().parse_args(["build"])) == 1
