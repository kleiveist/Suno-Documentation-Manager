from __future__ import annotations

import sys
from typing import TextIO

_EMOJI = {
    "OK": "✅",
    "SKIP": "⏭️",
    "WARN": "⚠️",
    "FAIL": "❌",
    "INFO": "ℹ️",
}


def _normalize(status: str) -> str:
    return status.strip().upper()


def format_message(status: str, message: str) -> str:
    normalized = _normalize(status)
    emoji = _EMOJI.get(normalized, _EMOJI["INFO"])
    return f"{emoji} {message}"


def status(status_value: str, message: str, *, stream: TextIO = sys.stdout) -> None:
    print(format_message(status_value, message), file=stream)


def ok(message: str, *, stream: TextIO = sys.stdout) -> None:
    status("OK", message, stream=stream)


def warn(message: str, *, stream: TextIO = sys.stdout) -> None:
    status("WARN", message, stream=stream)


def fail(message: str, *, stream: TextIO = sys.stderr) -> None:
    status("FAIL", message, stream=stream)


def info(message: str, *, stream: TextIO = sys.stdout) -> None:
    status("INFO", message, stream=stream)
