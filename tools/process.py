from __future__ import annotations

import os
import subprocess
import sys


def prepare_command(command: list[str]) -> list[str]:
    """Return a subprocess-safe command on every supported host.

    Node.js exposes npm, npx, and local package binaries as ``.cmd`` launchers
    on Windows. CreateProcess cannot execute those scripts directly, so route
    only that launcher type through the system command interpreter.
    """

    if sys.platform != "win32" or not command:
        return command
    if not command[0].lower().endswith((".cmd", ".bat")):
        return command

    command_processor = os.environ.get("COMSPEC", "cmd.exe")
    return [command_processor, "/d", "/s", "/c", subprocess.list2cmdline(command)]
