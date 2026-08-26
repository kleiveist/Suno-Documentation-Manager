from __future__ import annotations

import json
import re
import signal
import subprocess
import uuid
from pathlib import Path

import pytest

from tools import control
from tools.profiles import runtime as profile_runtime
from tools.tauri import common, doctor, paths, run
from tools.tauri import test as tauri_test
from tools.tauri.build import appimage, windows_portable
from tools.tauri.linux import install_arch

pytestmark = pytest.mark.skipif(
    not profile_runtime.feature_enabled("tauri"),
    reason="Tauri feature disabled by active profile",
)


def test_tauri_parser_recognizes_subcommands() -> None:
    parser = control._build_parser()

    cases = [
        ["tauri", "doctor", "--json"],
        ["tauri", "install", "--dry-run"],
        ["tauri", "install-appimage", "--dry-run"],
        ["tauri", "run", "--foreground", "--no-follow", "--frontend-port", "5174"],
        [
            "tauri",
            "build",
            "--target",
            "windows-portable",
            "--dry-run",
            "--bundles",
            "deb,rpm",
        ],
        ["tauri", "build", "--appimage", "--dry-run", "--skip-appimage-preflight"],
        ["tauri", "test", "--all"],
        ["tauri", "test", "--cargo"],
        ["tauri", "copy", "--dry-run", "--target-dir", ".dist/desktop"],
    ]

    for argv in cases:
        args = parser.parse_args(argv)
        assert args.command == "tauri"
        assert args.tauri_command == argv[1]


def test_tauri_identity_separates_display_and_artifact_names() -> None:
    payload = json.loads(paths.TAURI_CONFIG.read_text(encoding="utf-8"))

    assert paths.APP_NAME == "Suno Documentation Manager"
    assert paths.APP_ARTIFACT_NAME == "sunodm"
    assert paths.APP_SLUG == "sunodm"
    assert payload["productName"] == paths.APP_NAME
    assert payload["mainBinaryName"] == "sunodm"
    assert payload["app"]["windows"][0]["title"] == paths.APP_NAME
    wix_upgrade_code = payload["bundle"]["windows"]["wix"]["upgradeCode"]
    assert wix_upgrade_code == "54c9e875-02fa-5312-b9c4-14f17c9e3c61"
    assert wix_upgrade_code == str(uuid.uuid5(uuid.NAMESPACE_DNS, "sunodm.exe.app.x64"))


def test_bare_tauri_command_prints_help(capsys) -> None:
    code = control.main(["tauri"])

    assert code == 0
    output = capsys.readouterr().out
    assert "Tauri-specific diagnostics" in output
    assert "Tauri command map" in output
    assert "doctor" in output
    assert "install" in output


def test_tauri_command_aliases_are_normalized() -> None:
    assert control._normalize_argv(["tauri", "--doctor"]) == ["tauri", "doctor"]
    assert control._normalize_argv(["tauri", "--install", "--dry-run"]) == [
        "tauri",
        "install",
        "--dry-run",
    ]
    assert control._normalize_argv(["tauri", "--run", "--frontend-port", "5174"]) == [
        "tauri",
        "run",
        "--frontend-port",
        "5174",
    ]
    assert control._normalize_argv(["tauri", "--build", "--dry-run"]) == [
        "tauri",
        "build",
        "--dry-run",
    ]
    assert control._normalize_argv(["tauri", "--install-appimage", "--dry-run"]) == [
        "tauri",
        "install-appimage",
        "--dry-run",
    ]


def test_existing_aliases_do_not_collide_with_tauri() -> None:
    assert control._normalize_argv(["--doctor"]) == ["doctor"]
    assert control._normalize_argv(["--install"]) == ["install"]
    assert control._normalize_argv(["--run"]) == ["run"]
    assert control._normalize_argv(["--stop"]) == ["stop"]
    assert control._normalize_argv(["--test", "--suite", "frontend"]) == [
        "test",
        "--suite",
        "frontend",
    ]


def test_tauri_doctor_json_is_parseable(monkeypatch, capsys) -> None:
    monkeypatch.setattr(
        doctor,
        "collect_checks",
        lambda: ([common.CheckResult("fixture", "OK", "ready")], "OK"),
    )

    code = control.main(["tauri", "doctor", "--json"])

    assert code == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["overall"] == "OK"
    assert payload["checks"][0]["name"] == "fixture"


def test_tauri_install_dry_run_skips_mutating_commands(monkeypatch) -> None:
    commands: list[list[str]] = []

    def fake_run_command(command: list[str], **kwargs) -> common.CommandResult:
        commands.append(command)
        assert kwargs.get("dry_run") is True
        return common.CommandResult(command=command, cwd=paths.ROOT, returncode=0, dry_run=True)

    monkeypatch.setattr(common, "run_command", fake_run_command)
    monkeypatch.setattr("tools.tauri.install._ensure_rust", lambda dry_run: 0)
    monkeypatch.setattr("tools.tauri.install._ensure_node", lambda dry_run: 0)

    code = control.main(["tauri", "install", "--dry-run"])

    assert code == 0
    assert commands


def test_tauri_npm_install_uses_package_lock(monkeypatch) -> None:
    monkeypatch.setattr(common, "package_manager", lambda: "npm")
    monkeypatch.setattr(common.shutil, "which", lambda name: "/usr/bin/npm" if name == "npm" else None)

    assert common.package_manager_install_command() == [
        "/usr/bin/npm",
        "ci",
        "--no-audit",
        "--no-fund",
    ]


def test_tauri_cargo_checks_use_locked_dependencies(monkeypatch) -> None:
    commands: list[list[str]] = []

    monkeypatch.setattr(
        tauri_test.shutil,
        "which",
        lambda name: "/usr/bin/cargo" if name == "cargo" else None,
    )
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: (
            commands.append(command) or common.CommandResult(command=command, cwd=paths.ROOT, returncode=0)
        ),
    )

    assert tauri_test._run_cargo_checks() == 0
    assert [command[1] for command in commands] == ["check", "test"]
    assert all("--locked" in command for command in commands)
    assert all("--manifest-path" in command for command in commands)


def test_tauri_arch_install_uses_noninteractive_pacman(monkeypatch) -> None:
    commands: list[list[str]] = []

    def fake_run_command(command: list[str], **kwargs) -> common.CommandResult:
        commands.append(command)
        return common.CommandResult(command=command, cwd=paths.ROOT, returncode=0)

    monkeypatch.setattr(common, "run_command", fake_run_command)

    code = install_arch.install(dry_run=False)

    assert code == 0
    assert commands
    assert "--needed" in commands[0]
    assert "--noconfirm" in commands[0]


def test_tauri_build_dry_run_does_not_run_real_build(monkeypatch) -> None:
    calls: list[tuple[list[str], bool]] = []

    def fake_run_command(command: list[str], **kwargs) -> common.CommandResult:
        calls.append((command, bool(kwargs.get("dry_run"))))
        return common.CommandResult(command=command, cwd=paths.ROOT, returncode=0, dry_run=True)

    monkeypatch.setattr(common, "run_command", fake_run_command)

    code = control.main(["tauri", "build", "--target", "linux", "--dry-run"])

    assert code == 0
    assert calls
    assert calls[0][1] is True
    assert calls[0][0][-3:] == ["build", "--bundles", "deb,rpm,appimage"]


def test_tauri_build_plan_prints_target_command_and_bundles(monkeypatch) -> None:
    messages: list[str] = []
    monkeypatch.setattr(common.logger, "info", messages.append)

    common.print_build_plan(
        "linux",
        ["tauri", "build", "--bundles", "deb,rpm"],
        dry_run=True,
        bundles="deb,rpm",
    )

    assert "🧱 Build target: linux (dry-run)" in messages
    assert "📦 Bundles: deb,rpm" in messages
    assert "🛠️ Command: tauri build --bundles deb,rpm" in messages


def test_tauri_run_command_reports_missing_binary(monkeypatch) -> None:
    def fake_run(*args, **kwargs):
        raise FileNotFoundError("missing binary")

    monkeypatch.setattr(common.subprocess, "run", fake_run)

    result = common.run_command(["missing-command"])

    assert result.returncode == 127
    assert "missing binary" in result.stderr


def test_tauri_command_failure_reports_stdout_and_stderr_separately(monkeypatch) -> None:
    messages: list[str] = []
    monkeypatch.setattr(common.logger, "fail", messages.append)
    result = common.CommandResult(
        command=["cargo", "test"],
        cwd=paths.ROOT,
        returncode=1,
        stdout="test fixture failed\nassertion details",
        stderr="compiler context\nprocess exit details",
    )

    assert common.print_result(result, "passed", "failed") == 1
    assert messages == [
        "failed:\n"
        "command: cargo test\n"
        "stdout: test fixture failed | assertion details\n"
        "stderr: compiler context | process exit details"
    ]


def test_tauri_command_failure_emits_github_actions_annotation(monkeypatch, capsys) -> None:
    messages: list[str] = []
    monkeypatch.setattr(common.logger, "fail", messages.append)
    monkeypatch.setenv("GITHUB_ACTIONS", "true")
    result = common.CommandResult(
        command=["cargo", "test"],
        cwd=paths.ROOT,
        returncode=1,
        stdout="test fixture failed",
        stderr="process exit details",
    )

    assert common.print_result(result, "passed", "failed") == 1
    assert messages
    assert capsys.readouterr().out == (
        "::error title=Captured command failure::failed:%0A"
        "command: cargo test%0Astdout: test fixture failed%0A"
        "stderr: process exit details\n"
    )


def test_tauri_windows_portable_dry_run_uses_cargo_xwin_on_linux(monkeypatch) -> None:
    calls: list[tuple[list[str], bool]] = []
    messages: list[str] = []

    monkeypatch.setattr(common, "host_os", lambda: "linux")
    monkeypatch.setattr(
        "tools.tauri.build.windows_portable.shutil.which",
        lambda name, path=None: "cargo" if name == "cargo" else None,
    )
    monkeypatch.setattr(windows_portable.logger, "info", messages.append)
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: (
            calls.append((command, bool(kwargs.get("dry_run"))))
            or common.CommandResult(command, paths.ROOT, 0, dry_run=bool(kwargs.get("dry_run")))
        ),
    )

    code = control.main(["tauri", "build", "--target", "windows-portable", "--dry-run"])

    assert code == 0
    assert len(calls) == 3
    assert calls[0][1] is True
    assert calls[0][0][-3:] == ["cargo", "install", "cargo-xwin"]
    assert calls[1] == (["rustup", "component", "add", "llvm-tools-preview"], True)
    assert calls[2][1] is True
    assert "--runner" in calls[2][0]
    runner = calls[2][0][calls[2][0].index("--runner") + 1]
    assert Path(runner).name == "cargo-xwin"
    assert calls[2][0][-1:] == ["--no-bundle"]
    assert any(message.endswith(".dist/desktop/sunodm-windows-portable.zip") for message in messages)


def test_tauri_raw_windows_portable_flags_map_to_portable_target(monkeypatch) -> None:
    calls: list[tuple[list[str], bool]] = []

    monkeypatch.setattr(common, "host_os", lambda: "linux")
    monkeypatch.setattr(
        "tools.tauri.build.windows_portable.shutil.which",
        lambda name, path=None: "cargo" if name == "cargo" else None,
    )
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: (
            calls.append((command, bool(kwargs.get("dry_run"))))
            or common.CommandResult(command, paths.ROOT, 0, dry_run=bool(kwargs.get("dry_run")))
        ),
    )

    code = control.main(
        [
            "tauri",
            "build",
            "--runner",
            "cargo-xwin",
            "--target",
            "x86_64-pc-windows-msvc",
            "--no-bundle",
            "--dry-run",
        ]
    )

    assert code == 0
    assert len(calls) == 3
    assert calls[1] == (["rustup", "component", "add", "llvm-tools-preview"], True)
    assert calls[2][1] is True
    assert "--runner" in calls[2][0]
    runner = calls[2][0][calls[2][0].index("--runner") + 1]
    assert Path(runner).name == "cargo-xwin"
    assert calls[2][0][-1:] == ["--no-bundle"]


def test_tauri_windows_portable_installs_cargo_xwin_for_real_linux_build(
    monkeypatch,
) -> None:
    calls: list[tuple[list[str], bool]] = []
    installed = False
    llvm_installed = False

    def fake_which(name: str, path: str | None = None) -> str | None:
        if name == "cargo":
            return "/usr/bin/cargo"
        if name == "rustup":
            return "/usr/bin/rustup"
        if name == "cargo-xwin" and installed:
            return "fixture-bin/cargo-xwin"
        if name == "llvm-rc" and llvm_installed:
            return "fixture-toolchain/bin/llvm-rc"
        return None

    def fake_run_command(command: list[str], **kwargs) -> common.CommandResult:
        nonlocal installed, llvm_installed
        calls.append((command, bool(kwargs.get("dry_run"))))
        if command == ["/usr/bin/cargo", "install", "cargo-xwin"]:
            installed = True
        if command == ["/usr/bin/rustup", "component", "add", "llvm-tools-preview"]:
            llvm_installed = True
        return common.CommandResult(command, paths.ROOT, 0, dry_run=bool(kwargs.get("dry_run")))

    monkeypatch.setattr(common, "host_os", lambda: "linux")
    monkeypatch.setattr("tools.tauri.build.windows_portable.shutil.which", fake_which)
    monkeypatch.setattr(common, "run_command", fake_run_command)
    monkeypatch.setattr(
        "tools.tauri.build.windows_portable._zip_portable_binary",
        lambda dry_run=False: 0,
    )

    code = control.main(["tauri", "build", "--target", "windows-portable"])

    assert code == 0
    assert calls[0] == (["/usr/bin/cargo", "install", "cargo-xwin"], False)
    assert calls[1] == (
        ["/usr/bin/rustup", "component", "add", "llvm-tools-preview"],
        False,
    )
    assert "--runner" in calls[2][0]
    assert calls[2][0][calls[2][0].index("--runner") + 1] == "fixture-bin/cargo-xwin"
    assert calls[2][0][-1:] == ["--no-bundle"]


def test_tauri_windows_portable_fails_without_cargo_on_linux(monkeypatch) -> None:
    calls: list[list[str]] = []
    messages: list[str] = []

    monkeypatch.setattr(common, "host_os", lambda: "linux")
    monkeypatch.setattr("tools.tauri.build.windows_portable.shutil.which", lambda name, path=None: None)
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: calls.append(command) or common.CommandResult(command, paths.ROOT, 0),
    )
    monkeypatch.setattr("tools.tauri.build.windows_portable.logger.fail", messages.append)

    code = control.main(["tauri", "build", "--target", "windows-portable"])

    assert code == 1
    assert calls == []
    assert any("cargo not found" in message for message in messages)


def test_tauri_windows_cross_dry_run_requires_linux_host(monkeypatch) -> None:
    calls: list[list[str]] = []
    messages: list[str] = []

    monkeypatch.setattr(common, "host_os", lambda: "windows")
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: calls.append(command) or common.CommandResult(command, paths.ROOT, 0),
    )
    monkeypatch.setattr("tools.tauri.build.windows_cross_linux.logger.fail", messages.append)

    code = control.main(["tauri", "build", "--target", "windows-cross-linux", "--dry-run"])

    assert code == 1
    assert calls == []
    assert any("Windows cross-build is supported from Linux hosts only" in message for message in messages)


def test_tauri_appimage_fallback_removes_legacy_long_name_entries(tmp_path) -> None:
    appdir = tmp_path / "sunodm.AppDir"
    current_binary = appdir / "usr" / "bin" / paths.APP_ARTIFACT_NAME
    legacy_binary = appdir / "usr" / "bin" / paths.APP_DISPLAY_SLUG
    current_desktop = appdir / "usr" / "share" / "applications" / f"{paths.APP_ARTIFACT_NAME}.desktop"
    legacy_desktop = appdir / "usr" / "share" / "applications" / f"{paths.APP_NAME}.desktop"
    current_binary.parent.mkdir(parents=True)
    current_desktop.parent.mkdir(parents=True)
    current_binary.write_bytes(b"current")
    legacy_binary.write_bytes(b"legacy")
    current_desktop.write_text("current", encoding="utf-8")
    legacy_desktop.write_text("legacy", encoding="utf-8")

    appimage._cleanup_legacy_appdir(appdir)

    assert current_binary.read_bytes() == b"current"
    assert current_desktop.read_text(encoding="utf-8") == "current"
    assert not legacy_binary.exists()
    assert not legacy_desktop.exists()


def test_tauri_build_fails_when_frontend_dependencies_are_missing(monkeypatch) -> None:
    calls: list[list[str]] = []

    monkeypatch.setattr(common, "frontend_dependencies_ready", lambda: False)
    monkeypatch.setattr(
        common,
        "missing_frontend_dependency_paths",
        lambda: [paths.FRONTEND_DIR / "node_modules" / "@types" / "node"],
    )
    monkeypatch.setattr(
        common,
        "run_command",
        lambda command, **kwargs: calls.append(command) or common.CommandResult(command, paths.ROOT, 0),
    )

    code = control.main(["tauri", "build", "--target", "linux"])

    assert code == 1
    assert calls == []


def test_tauri_cli_fallback_uses_tauri_apps_cli_package(monkeypatch) -> None:
    monkeypatch.setattr(paths, "local_tauri_binary", lambda: paths.FRONTEND_DIR / "missing-tauri")
    monkeypatch.setattr(common, "package_manager", lambda: "npm")

    command = common.tauri_cli_command("dev")

    assert Path(command[0]).name == "npm"
    assert command[1:5] == ["exec", "--yes", "--package", "@tauri-apps/cli@2.11.4"]
    assert command[-2:] == ["tauri", "dev"]


def test_tauri_commands_remove_server_only_environment(monkeypatch) -> None:
    captured: dict[str, str] = {}
    monkeypatch.setenv("DATABASE_URL", "postgresql+psycopg://app:secret@localhost/app")
    monkeypatch.setenv("SECRET_KEY", "application-secret")
    monkeypatch.setenv("CARGO_REGISTRY_TOKEN", "tooling-token")

    def fake_run(command: list[str], **kwargs) -> subprocess.CompletedProcess[str]:
        captured.update(kwargs["env"])
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(common.subprocess, "run", fake_run)

    assert common.run_command(["tauri", "--version"]).returncode == 0
    assert "DATABASE_URL" not in captured
    assert "SECRET_KEY" not in captured
    assert captured["CARGO_REGISTRY_TOKEN"] == "tooling-token"


def test_tauri_run_override_uses_frontend_cwd_command() -> None:
    payload = json.loads(run._dev_config_override(5174))

    assert payload["build"]["beforeDevCommand"] == "cd ../frontend && npm run dev -- --host 127.0.0.1 --port 5174"
    assert payload["build"]["devUrl"] == "http://127.0.0.1:5174"


def test_tauri_run_defaults_to_detached(monkeypatch, capsys) -> None:
    started: list[list[str]] = []

    monkeypatch.setattr(common, "tauri_cli_command", lambda *args: ["tauri", *args])
    monkeypatch.setattr(
        run,
        "_run_detached",
        lambda command, follow=True, **_kwargs: started.append([*command, f"follow={follow}"]) or 0,
    )

    code = control.main(["tauri", "run"])

    assert code == 0
    assert started == [["tauri", "dev", "--config", run._dev_config_override(5173), "follow=True"]]
    assert capsys.readouterr().err == ""


def test_tauri_run_no_follow_returns_after_background_start(monkeypatch) -> None:
    started: list[list[str]] = []

    monkeypatch.setattr(common, "tauri_cli_command", lambda *args: ["tauri", *args])
    monkeypatch.setattr(
        run,
        "_run_detached",
        lambda command, follow=True, **_kwargs: started.append([*command, f"follow={follow}"]) or 0,
    )

    code = control.main(["tauri", "run", "--no-follow"])

    assert code == 0
    assert started == [["tauri", "dev", "--config", run._dev_config_override(5173), "follow=False"]]


def test_tauri_run_foreground_uses_current_terminal(monkeypatch) -> None:
    calls: list[list[str]] = []

    monkeypatch.setattr(common, "tauri_cli_command", lambda *args: ["tauri", *args])
    monkeypatch.setattr(
        run,
        "_run_detached",
        lambda command: (_ for _ in ()).throw(AssertionError("should not detach")),
    )

    def fake_run_command(command: list[str], **kwargs) -> common.CommandResult:
        calls.append(command)
        return common.CommandResult(command=command, cwd=paths.ROOT, returncode=0)

    monkeypatch.setattr(common, "run_command", fake_run_command)

    code = control.main(["tauri", "run", "--foreground"])

    assert code == 0
    assert calls == [["tauri", "dev", "--config", run._dev_config_override(5173)]]


def test_tauri_follow_ctrl_c_stops_process_group(monkeypatch, tmp_path) -> None:
    log_path = tmp_path / "tauri.log"
    log_path.write_text("ready\n", encoding="utf-8")
    killed: list[tuple[int, int]] = []

    class FakeProcess:
        pid = 4321

        def __init__(self) -> None:
            self.poll_count = 0
            self.terminated = False

        def poll(self) -> int | None:
            self.poll_count += 1
            if self.poll_count == 1:
                raise KeyboardInterrupt
            if self.terminated:
                return 0
            return None

    monkeypatch.setattr(run, "_clear_state", lambda: None)
    fake_process = FakeProcess()

    def fake_killpg(pid: int, sig: int) -> None:
        killed.append((pid, sig))
        fake_process.terminated = True

    monkeypatch.setattr(run.os, "killpg", fake_killpg)

    code = run._follow_log(log_path, fake_process)  # type: ignore[arg-type]

    assert code == 0
    assert killed == [(4321, signal.SIGTERM)]


def test_tauri_package_has_no_bare_imports_or_legacy_tokens() -> None:
    legacy_pattern = re.compile(r"FMDFlashcard|fmdflashcard|fmd-desktop|apps/fmd-desktop|com\.fmd\.flashcard|AppInsall")
    bare_import_pattern = re.compile(r"from\s+(doctor|console|installuix|installuixubu)\s+import")

    scanned = list((paths.ROOT / "tools" / "tauri").rglob("*.py"))
    scanned.extend((paths.ROOT / "src-tauri").rglob("*"))

    text_suffixes = {".json", ".md", ".py", ".rs", ".svg", ".toml", ".txt"}
    offenders: list[str] = []
    for path in scanned:
        relative = path.relative_to(paths.ROOT)
        if any(part in {"target", "gen", "binaries"} for part in relative.parts):
            continue
        if not path.is_file() or (path.suffix not in text_suffixes and path.name != "Cargo.lock"):
            continue
        text = path.read_text(encoding="utf-8")
        if legacy_pattern.search(text) or bare_import_pattern.search(text):
            offenders.append(str(relative))

    assert offenders == []
