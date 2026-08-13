from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import time
from pathlib import Path

from tools import logger
from tools.config import ConfigLoadError, is_server_only_name, resolve_configuration, validate_configuration
from tools.profiles import runtime as profile_runtime
from tools.tauri import common, paths

RUNTIME_DIR = paths.ROOT / "tools" / ".runtime"
LOG_DIR = RUNTIME_DIR / "logs"
STATE_FILE = RUNTIME_DIR / "tauri_run_state.json"


def main(args: argparse.Namespace) -> int:
    profile = profile_runtime.active_profile(paths.ROOT)
    try:
        resolved = resolve_configuration(
            profile,
            project_root=paths.ROOT,
            cli_overrides={
                "FRONTEND_HOST": getattr(args, "frontend_host", None),
                "FRONTEND_PORT": getattr(args, "frontend_port", None),
            },
        )
    except ConfigLoadError as exc:
        logger.fail(f"Could not load frontend configuration: {exc}")
        return 1
    relevant = {"FRONTEND_HOST", "FRONTEND_PORT"}
    issues = [issue for issue in validate_configuration(resolved) if issue.name in relevant]
    if issues:
        for issue in issues:
            logger.fail(f"{issue.name}: {issue.message}")
        return 1
    frontend_host = resolved.value("FRONTEND_HOST")
    frontend_port_value = resolved.value("FRONTEND_PORT")
    assert frontend_host is not None and frontend_port_value is not None
    frontend_port = int(frontend_port_value)
    command = common.tauri_cli_command(
        "dev",
        "--config",
        _dev_config_override(frontend_port, frontend_host),
    )
    frontend_names = {
        "FRONTEND_HOST",
        "FRONTEND_PORT",
        "BACKEND_HOST",
        "BACKEND_PORT",
        "VITE_API_BASE_URL",
    }
    frontend_environment = {
        name: value
        for name, value in resolved.values.items()
        if value is not None and name in frontend_names
    }
    secret_environment = {name for name in os.environ if is_server_only_name(name)}

    if getattr(args, "foreground", False):
        result = common.run_command(
            command,
            cwd=paths.ROOT,
            env=frontend_environment,
            remove_env=secret_environment,
        )
        return common.print_result(result, "Tauri dev session finished", "Tauri dev failed")

    return _run_detached(
        command,
        follow=not bool(getattr(args, "no_follow", False)),
        env=frontend_environment,
        remove_env=secret_environment,
    )


def _dev_config_override(frontend_port: int, frontend_host: str = "127.0.0.1") -> str:
    client_host = "127.0.0.1" if frontend_host in {"0.0.0.0", "::", "[::]"} else frontend_host
    return json.dumps(
        {
            "build": {
                "beforeDevCommand": (
                    "cd ../frontend && npm run dev -- "
                    f"--host {frontend_host} --port {frontend_port}"
                ),
                "devUrl": f"http://{client_host}:{frontend_port}",
            }
        }
    )


def _run_detached(
    command: list[str],
    *,
    follow: bool = True,
    env: dict[str, str] | None = None,
    remove_env: set[str] | None = None,
) -> int:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    log_path = LOG_DIR / "tauri.log"
    log_file = log_path.open("w", encoding="utf-8")
    environment = {
        name: value
        for name, value in os.environ.items()
        if not is_server_only_name(name)
    }
    for name in remove_env or set():
        environment.pop(name, None)
    environment.update(env or {})
    process = subprocess.Popen(
        command,
        cwd=paths.ROOT,
        env=environment,
        stdout=log_file,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    log_file.close()
    _write_detached_state(process.pid, log_path)
    logger.ok(f"Tauri dev started in background pid={process.pid} log={log_path}")
    if not follow:
        logger.info(f"Follow logs with: tail -f {log_path.relative_to(paths.ROOT)}")
        return 0
    return _follow_log(log_path, process)


def _follow_log(log_path: Path, process: subprocess.Popen) -> int:
    logger.info("Streaming Tauri log. Press Ctrl+C to stop Tauri.")
    try:
        with log_path.open("r", encoding="utf-8", errors="replace") as handle:
            while True:
                line = handle.readline()
                if line:
                    print(line, end="", flush=True)
                    continue

                returncode = process.poll()
                if returncode is not None:
                    rest = handle.read()
                    if rest:
                        print(rest, end="", flush=True)
                    if returncode == 0:
                        logger.ok("Tauri dev process exited")
                        return 0
                    logger.fail(f"Tauri dev process exited with code {returncode}")
                    return int(returncode)

                time.sleep(0.2)
    except KeyboardInterrupt:
        print()
        stopped = _terminate_process_group(process)
        _clear_state()
        if stopped:
            logger.ok(f"Tauri dev stopped pid={process.pid}")
        else:
            logger.fail(f"Tauri dev did not stop cleanly pid={process.pid}")
        return 0
    return 0


def _write_detached_state(pid: int, log_path: Path) -> None:
    RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(
        f'{{"pid": {pid}, "log": "{log_path}", "command": "tauri dev"}}\n',
        encoding="utf-8",
    )


def _clear_state() -> None:
    try:
        STATE_FILE.unlink(missing_ok=True)
    except OSError:
        pass


def _terminate_process_group(process: subprocess.Popen, *, timeout_seconds: float = 8.0) -> bool:
    if process.poll() is not None:
        return True

    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return True
    except OSError:
        try:
            process.terminate()
        except OSError:
            return True

    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if process.poll() is not None:
            return True
        time.sleep(0.2)

    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        return True
    except OSError:
        try:
            process.kill()
        except OSError:
            return True

    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if process.poll() is not None:
            return True
        time.sleep(0.1)
    return process.poll() is not None
