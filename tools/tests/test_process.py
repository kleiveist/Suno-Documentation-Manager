from __future__ import annotations

from tools import process


def test_prepare_command_keeps_native_executable(monkeypatch) -> None:
    monkeypatch.setattr(process.sys, "platform", "win32")

    command = [r"C:\Python311\python.exe", "--version"]

    assert process.prepare_command(command) is command


def test_prepare_command_routes_windows_batch_launcher_through_comspec(monkeypatch) -> None:
    monkeypatch.setattr(process.sys, "platform", "win32")
    monkeypatch.setenv("COMSPEC", r"C:\Windows\System32\cmd.exe")

    prepared = process.prepare_command(
        [r"C:\Program Files\nodejs\npm.cmd", "ci", "--no-audit"]
    )

    assert prepared[:4] == [r"C:\Windows\System32\cmd.exe", "/d", "/s", "/c"]
    assert prepared[4] == '"C:\\Program Files\\nodejs\\npm.cmd" ci --no-audit'
