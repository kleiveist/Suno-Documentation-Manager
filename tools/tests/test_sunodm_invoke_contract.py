from __future__ import annotations

import json
import re
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_COMMAND_COUNT = 54
EXPECTED_COMMANDS = frozenset(
    {
        "add_deviation",
        "adopt_legacy_profile",
        "attach_configured_external_timestamp",
        "attach_global_evidence",
        "calculate_hashes",
        "create_album",
        "create_revision",
        "create_track",
        "create_workspace",
        "execute_folder_import",
        "finalize_track",
        "generate_artwork_disclosure",
        "generate_documents",
        "get_audio_screening_settings",
        "get_profile",
        "get_timestamp_settings",
        "get_workflow",
        "import_evidence",
        "import_global_evidence",
        "import_global_terms_evidence",
        "invalidate_certificate",
        "list_albums",
        "list_global_evidence",
        "list_tracks",
        "load_track",
        "load_track_cover",
        "open_workspace",
        "open_workspace_by_path",
        "preview_documents",
        "preview_evidence",
        "re_evaluate_track",
        "remove_deviation",
        "remove_evidence",
        "remove_global_evidence",
        "rename_album",
        "resolve_deviation",
        "run_external_audio_screening",
        "run_local_audio_screening",
        "scan_import_folder",
        "scan_workspace",
        "set_step_status",
        "test_audio_screening_provider",
        "test_timestamp_provider",
        "update_audio_screening_secret",
        "update_audio_screening_settings",
        "update_global_terms_evidence_metadata",
        "update_profile",
        "update_timestamp_secret",
        "update_timestamp_settings",
        "update_track",
        "update_track_library",
        "validate_track",
        "verify_evidence",
        "verify_hashes",
    }
)


def _read(relative_path: str) -> str:
    return (ROOT / relative_path).read_text(encoding="utf-8")


def _json(relative_path: str) -> dict[str, object]:
    return json.loads(_read(relative_path))


def _toml(relative_path: str) -> dict[str, object]:
    return tomllib.loads(_read(relative_path))


def _rust_command_functions() -> list[str]:
    return re.findall(
        r"#\[tauri::command(?:\([^\]]*\))?\]\s+pub\s+(?:async\s+)?fn\s+([a-z][a-z0-9_]*)",
        _read("src-tauri/src/commands.rs"),
    )


def _registered_commands() -> list[str]:
    source = _read("src-tauri/src/main.rs")
    handler = re.search(r"tauri::generate_handler!\[\s*(.*?)\s*\]", source, re.DOTALL)
    assert handler is not None, "src-tauri/src/main.rs has no generate_handler! registration"
    return re.findall(r"\bcommands::([a-z][a-z0-9_]*)\b", handler.group(1))


def _frontend_invoke_names() -> list[str]:
    return re.findall(
        r"\bcommand(?:<[^>]+>)?\(\s*[\"']([a-z][a-z0-9_]*)[\"']",
        _read("frontend/src/api/desktop.ts"),
    )


def test_all_sunodm_invoke_surfaces_expose_the_exact_command_contract() -> None:
    surfaces = {
        "Rust #[tauri::command] functions": _rust_command_functions(),
        "Rust generate_handler! registrations": _registered_commands(),
        "TypeScript invoke names": _frontend_invoke_names(),
    }

    assert len(EXPECTED_COMMANDS) == EXPECTED_COMMAND_COUNT
    for name, commands in surfaces.items():
        assert len(commands) == EXPECTED_COMMAND_COUNT, f"{name} count drifted: {commands}"
        assert len(commands) == len(set(commands)), f"{name} contains duplicates: {commands}"
        assert set(commands) == EXPECTED_COMMANDS, f"{name} no longer matches the SunoDM contract"

    command_sets = [set(commands) for commands in surfaces.values()]
    assert command_sets[0] == command_sets[1] == command_sets[2]


def test_sunodm_desktop_identity_and_version_are_consistent() -> None:
    version = _read("VERSION").strip()
    profile = _toml("project-profile.toml")
    tauri = _json("src-tauri/tauri.conf.json")
    cargo = _toml("src-tauri/Cargo.toml")
    cargo_lock = _toml("src-tauri/Cargo.lock")
    frontend = _json("frontend/package.json")
    frontend_lock = _json("frontend/package-lock.json")

    assert version == "0.1.0"
    assert profile["id"] == "desktop-local"
    assert profile["features"] == ["frontend", "tauri"]
    assert profile["optional_features"] == []

    assert tauri["productName"] == "sunodm"
    assert tauri["identifier"] == "com.grav0id.sunodoc"
    assert tauri["mainBinaryName"] == "sunodm"
    assert tauri["version"] == version
    assert tauri["app"]["windows"][0]["title"] == "Suno Documentation Manager"

    assert cargo["package"]["name"] == "sunodm"
    assert cargo["package"]["version"] == version
    sunodm_lock_entries = [package for package in cargo_lock["package"] if package["name"] == "sunodm"]
    assert len(sunodm_lock_entries) == 1
    assert sunodm_lock_entries[0]["version"] == version

    assert frontend["name"] == "sunodm-frontend"
    assert frontend["version"] == version
    assert frontend_lock["name"] == frontend["name"]
    assert frontend_lock["version"] == version
    assert frontend_lock["packages"][""]["name"] == frontend["name"]
    assert frontend_lock["packages"][""]["version"] == version
    assert "<title>Suno Documentation Manager</title>" in _read("frontend/index.html")


def test_sunodm_uses_the_minimal_local_desktop_capability_without_a_backend() -> None:
    tauri = _json("src-tauri/tauri.conf.json")
    capability = _json("src-tauri/capabilities/default.json")
    windows = tauri["app"]["windows"]

    assert [window["label"] for window in windows] == ["main"]
    assert capability["identifier"] == "default"
    assert capability["windows"] == ["main"]
    assert capability["permissions"] == ["core:default"]
    assert "remote" not in capability

    assert not (ROOT / "backend").exists()
    assert not (ROOT / "deployment" / "compose.yaml").exists()
    assert not (ROOT / "frontend" / "src" / "api" / "backend.ts").exists()
