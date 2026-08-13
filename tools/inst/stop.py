from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import time
from pathlib import Path

from tools import logger
from tools.config import ConfigLoadError, resolve_configuration, validate_configuration
from tools.profiles import runtime as profile_runtime

ROOT = Path(__file__).resolve().parents[2]
RUNTIME_DIR = ROOT / "tools" / ".runtime"
STATE_FILE = RUNTIME_DIR / "run_state.json"


def _is_process_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def _terminate_pid(pid: int, timeout_seconds: int = 8) -> bool:
    if not _is_process_alive(pid):
        return True

    try:
        os.kill(pid, signal.SIGTERM)
    except OSError:
        return True

    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        if not _is_process_alive(pid):
            return True
        time.sleep(0.2)

    try:
        os.kill(pid, signal.SIGKILL)
    except OSError:
        return True

    return not _is_process_alive(pid)


def _read_state() -> dict | None:
    if not STATE_FILE.exists():
        return None
    try:
        return json.loads(STATE_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def _clear_state() -> None:
    try:
        STATE_FILE.unlink(missing_ok=True)
    except OSError:
        pass


def _read_cmdline(pid: int) -> str:
    path = Path("/proc") / str(pid) / "cmdline"
    try:
        raw = path.read_bytes()
    except OSError:
        return "(unreadable)"
    text = raw.replace(b"\x00", b" ").decode(errors="replace").strip()
    return text or "(empty)"


def _listener_inode_to_port(ports: set[int]) -> dict[str, int]:
    mapping: dict[str, int] = {}
    tcp_tables = [Path("/proc/net/tcp"), Path("/proc/net/tcp6")]

    for table in tcp_tables:
        if not table.exists():
            continue
        try:
            lines = table.read_text(encoding="utf-8").splitlines()
        except OSError:
            continue
        for line in lines[1:]:
            parts = line.split()
            if len(parts) < 10:
                continue
            local_addr = parts[1]
            state = parts[3]
            inode = parts[9]
            try:
                port = int(local_addr.split(":")[1], 16)
            except (IndexError, ValueError):
                continue
            if state != "0A" or port not in ports:
                continue
            mapping[inode] = port

    return mapping


def _port_owners_from_proc_sockets(ports: set[int]) -> dict[int, tuple[set[int], str]]:
    inode_to_port = _listener_inode_to_port(ports)
    if not inode_to_port:
        return {}

    owners: dict[int, tuple[set[int], str]] = {}
    for pid_name in os.listdir("/proc"):
        if not pid_name.isdigit():
            continue
        pid = int(pid_name)
        fd_dir = Path("/proc") / pid_name / "fd"
        if not fd_dir.exists():
            continue
        pid_ports: set[int] = set()
        try:
            fd_names = list(fd_dir.iterdir())
        except OSError:
            continue
        for fd in fd_names:
            try:
                target = os.readlink(fd)
            except OSError:
                continue
            if not target.startswith("socket:[") or not target.endswith("]"):
                continue
            inode = target[8:-1]
            port = inode_to_port.get(inode)
            if port is not None:
                pid_ports.add(port)
        if pid_ports:
            owners[pid] = (pid_ports, _read_cmdline(pid))
    return owners


def _parse_port(value: str) -> int | None:
    candidate = value.strip()
    if not candidate:
        return None
    if candidate.startswith("[") and "]:" in candidate:
        candidate = candidate.rsplit("]:", 1)[1]
    elif ":" in candidate:
        candidate = candidate.rsplit(":", 1)[1]
    try:
        return int(candidate)
    except ValueError:
        return None


def _port_owners_from_ss(ports: set[int]) -> dict[int, tuple[set[int], str]]:
    ss = shutil.which("ss")
    if ss is None:
        return {}

    completed = subprocess.run([ss, "-lptn"], text=True, capture_output=True, check=False)
    if completed.returncode != 0:
        return {}

    owners: dict[int, tuple[set[int], str]] = {}
    for line in completed.stdout.splitlines():
        parts = line.split()
        if len(parts) < 6:
            continue
        port = _parse_port(parts[3])
        if port is None or port not in ports:
            continue

        tail = " ".join(parts[5:])
        pid_parts = [chunk for chunk in tail.replace(",", " ").split() if chunk.startswith("pid=")]
        for chunk in pid_parts:
            try:
                pid = int(chunk.split("=", 1)[1])
            except ValueError:
                continue

            current_ports, _ = owners.get(pid, (set(), ""))
            current_ports.add(port)
            owners[pid] = (current_ports, _read_cmdline(pid))
    return owners


def _cmdline_matches_port(tokens: list[str], port: int) -> bool:
    target = str(port)
    if not tokens:
        return False

    for idx, token in enumerate(tokens):
        if token == "--port" and idx + 1 < len(tokens) and tokens[idx + 1] == target:
            return True
        if token.startswith("--port=") and token.split("=", 1)[1] == target:
            return True
        if token.endswith(f":{target}"):
            return True

    joined = " ".join(tokens)
    if ("uvicorn" in joined or "http.server" in joined or "vite" in joined) and target in tokens:
        return True
    return False


def _port_owners_from_cmdline(ports: set[int]) -> dict[int, tuple[set[int], str]]:
    owners: dict[int, tuple[set[int], str]] = {}
    for pid_name in os.listdir("/proc"):
        if not pid_name.isdigit():
            continue
        pid = int(pid_name)
        cmdline_path = Path("/proc") / pid_name / "cmdline"
        try:
            raw = cmdline_path.read_bytes()
        except OSError:
            continue
        if not raw:
            continue
        tokens = [part.decode(errors="replace") for part in raw.split(b"\x00") if part]
        if not tokens:
            continue

        pid_ports = {port for port in ports if _cmdline_matches_port(tokens, port)}
        if pid_ports:
            owners[pid] = (pid_ports, " ".join(tokens))

    return owners


def _merge_owners(base: dict[int, tuple[set[int], str]], extra: dict[int, tuple[set[int], str]]) -> None:
    for pid, (ports, cmdline) in extra.items():
        existing_ports, existing_cmd = base.get(pid, (set(), ""))
        merged_ports = set(existing_ports)
        merged_ports.update(ports)
        merged_cmd = existing_cmd or cmdline
        base[pid] = (merged_ports, merged_cmd)


def _port_owners(ports: set[int]) -> dict[int, tuple[set[int], str]]:
    owners: dict[int, tuple[set[int], str]] = {}
    _merge_owners(owners, _port_owners_from_proc_sockets(ports))
    _merge_owners(owners, _port_owners_from_ss(ports))
    if not owners:
        _merge_owners(owners, _port_owners_from_cmdline(ports))
    return owners


def _port_is_free(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind(("127.0.0.1", port))
            return True
        except OSError:
            return False


def _stop_tracked_processes() -> tuple[set[int], int]:
    state = _read_state()
    stopped_pids: set[int] = set()
    failures = 0

    if not state or "services" not in state:
        logger.ok("No tracked services are running")
        _clear_state()
        return stopped_pids, failures

    for service in state.get("services", []):
        pid = int(service.get("pid", -1))
        name = str(service.get("name", "unknown"))
        if pid <= 0:
            continue
        ok = _terminate_pid(pid)
        stopped_pids.add(pid)
        logger.status("OK" if ok else "FAIL", f"stop:tracked:{name:<8} pid={pid}")
        if not ok:
            failures += 1

    _clear_state()
    return stopped_pids, failures


def _stop_port_processes(ports: set[int], ignored_pids: set[int]) -> int:
    owners = _port_owners(ports)
    if not owners:
        still_occupied = [port for port in sorted(ports) if not _port_is_free(port)]
        if still_occupied:
            logger.fail(
                "Ports are occupied but no owner process could be resolved automatically: "
                f"{still_occupied}. Action: run 'sudo ss -lptn \"sport = :<port>\"' and stop the owner process."
            )
            return len(still_occupied)
        logger.ok(f"No listener process found on ports {sorted(ports)}")
        return 0

    failures = 0
    current_pid = os.getpid()
    for pid, (pid_ports, cmdline) in sorted(owners.items(), key=lambda item: item[0]):
        if pid in ignored_pids or pid == current_pid:
            continue
        ok = _terminate_pid(pid)
        port_text = ",".join(str(port) for port in sorted(pid_ports))
        short_cmd = (cmdline[:120] + "...") if len(cmdline) > 120 else cmdline
        logger.status("OK" if ok else "FAIL", f"stop:port:{port_text:<9} pid={pid} cmd={short_cmd}")
        if not ok:
            failures += 1

    still_occupied = [port for port in sorted(ports) if not _port_is_free(port)]
    if still_occupied:
        failures += len(still_occupied)
        logger.fail(
            "Unable to clear ports automatically: "
            f"{still_occupied}. Action: run 'sudo ss -lptn \"sport = :<port>\"' and stop the owner process."
        )

    return failures


def main(args: argparse.Namespace) -> int:
    tracked_stopped, failures = _stop_tracked_processes()
    if not args.tracked_only:
        profile = profile_runtime.active_profile(ROOT)
        try:
            resolved = resolve_configuration(
                profile,
                project_root=ROOT,
                cli_overrides={
                    "FRONTEND_PORT": getattr(args, "frontend_port", None),
                    "BACKEND_PORT": getattr(args, "backend_port", None),
                },
            )
        except ConfigLoadError as exc:
            logger.fail(f"Could not load development ports: {exc}")
            return 1
        relevant_names = {"FRONTEND_PORT", "BACKEND_PORT"}
        relevant_issues = [issue for issue in validate_configuration(resolved) if issue.name in relevant_names]
        if relevant_issues:
            for issue in relevant_issues:
                logger.fail(f"{issue.name}: {issue.message}")
            return 1
        ports: set[int] = set()
        if profile.has_feature("frontend"):
            frontend_port = resolved.value("FRONTEND_PORT")
            assert frontend_port is not None
            ports.add(int(frontend_port))
        if profile.has_feature("backend"):
            backend_port = resolved.value("BACKEND_PORT")
            assert backend_port is not None
            ports.add(int(backend_port))
        if ports:
            failures += _stop_port_processes(ports, tracked_stopped)
        else:
            logger.ok("Active profile has no development service ports to inspect")

    if failures == 0:
        logger.ok("Stop completed")
        return 0

    logger.fail(f"Stop completed with {failures} failure(s)")
    return 1
