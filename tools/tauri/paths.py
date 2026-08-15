from __future__ import annotations

import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FRONTEND_DIR = ROOT / "frontend"
TAURI_DIR = ROOT / "src-tauri"
DIST_DIR = ROOT / ".dist" / "desktop"

FRONTEND_PACKAGE_JSON = FRONTEND_DIR / "package.json"
FRONTEND_PACKAGE_LOCK = FRONTEND_DIR / "package-lock.json"
FRONTEND_PNPM_LOCK = FRONTEND_DIR / "pnpm-lock.yaml"
TAURI_CONFIG = TAURI_DIR / "tauri.conf.json"


def _app_identity() -> tuple[str, str, str]:
    fallback = ("Template Project", "project-template", "com.example.templateproject")
    try:
        payload = json.loads(TAURI_CONFIG.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return fallback
    artifact_name = payload.get("mainBinaryName") or payload.get("productName")
    identifier = payload.get("identifier")
    display_name: object = None
    app = payload.get("app")
    if isinstance(app, dict):
        windows = app.get("windows")
        if isinstance(windows, list):
            main_window = next(
                (window for window in windows if isinstance(window, dict) and window.get("label") == "main"),
                None,
            )
            if isinstance(main_window, dict):
                display_name = main_window.get("title")
    if not isinstance(display_name, str) or not display_name.strip():
        display_name = payload.get("productName")
    return (
        display_name if isinstance(display_name, str) and display_name.strip() else fallback[0],
        artifact_name if isinstance(artifact_name, str) and artifact_name.strip() else fallback[1],
        identifier if isinstance(identifier, str) and identifier.strip() else fallback[2],
    )


def _slug(value: str) -> str:
    normalized = re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")
    return normalized or "template-project"


APP_NAME, APP_ARTIFACT_NAME, APP_ID = _app_identity()
APP_ARTIFACT_NAME = _slug(APP_ARTIFACT_NAME)
APP_SLUG = APP_ARTIFACT_NAME
APP_DISPLAY_SLUG = _slug(APP_NAME)


def local_tauri_binary() -> Path:
    binary_name = "tauri.cmd" if os.name == "nt" else "tauri"
    return FRONTEND_DIR / "node_modules" / ".bin" / binary_name


def bundle_roots() -> list[Path]:
    roots = [TAURI_DIR / "target" / "release" / "bundle"]
    target_root = TAURI_DIR / "target"
    if target_root.exists():
        roots.extend(sorted(target_root.glob("*/release/bundle")))
    return roots
