from __future__ import annotations

import argparse

from tools import logger
from tools.profiles import runtime as profile_runtime
from tools.tauri import build, copy, doctor, install, run, test
from tools.tauri.build import installappimage
from tools.tauri import paths


class TauriHelpFormatter(argparse.RawDescriptionHelpFormatter):
    def __init__(self, prog: str) -> None:
        super().__init__(prog, max_help_position=30, width=100)


def configure_build_parser(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--target",
        default="linux",
        metavar="{linux,windows,windows-portable,windows-cross-linux,macos}",
        help="platform strategy (default: linux)",
    )
    parser.add_argument("--runner", help=argparse.SUPPRESS)
    parser.add_argument("--no-bundle", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--dry-run", action="store_true", help="show the build plan without changing files")
    parser.add_argument("--no-clean", action="store_true", help="keep previous Tauri build output")
    parser.add_argument(
        "--bundles",
        help="Linux bundle list, for example deb,rpm or appimage (default: deb,rpm,appimage)",
    )
    parser.add_argument(
        "--appimage",
        action="store_true",
        help="build and locally install only the Linux AppImage",
    )
    parser.add_argument(
        "--skip-appimage-preflight",
        action="store_true",
        help="skip AppImage host-tool checks (the build may still fail)",
    )


def configure_parser(parser: argparse.ArgumentParser) -> None:
    parser.set_defaults(tauri_parser=parser)
    parser.epilog = """
recommended return-to-project workflow:
  python tools/control.py tauri doctor
  python tools/control.py tauri install --dry-run
  python tools/control.py tauri run --foreground
  python tools/control.py tauri build --dry-run

Use '<command> --help' before an unfamiliar or destructive operation.
""".strip()
    tauri_subparsers = parser.add_subparsers(
        dest="tauri_command",
        title="Tauri command map",
        metavar="<command>",
    )

    doctor_parser = tauri_subparsers.add_parser(
        "doctor",
        help="inspect desktop prerequisites",
        description="Check Rust, the Tauri CLI, platform libraries, ports and scaffold files.",
        formatter_class=TauriHelpFormatter,
    )
    doctor_parser.add_argument("--json", action="store_true", help="print machine-readable JSON")
    doctor_parser.add_argument("--watch", action="store_true", help="repeat checks until interrupted")
    doctor_parser.add_argument("--interval", type=int, default=5, help="seconds between checks (default: 5)")
    doctor_parser.epilog = "examples:\n  python tools/control.py tauri doctor\n  python tools/control.py tauri doctor --json"

    install_parser = tauri_subparsers.add_parser(
        "install",
        help="prepare desktop dependencies",
        description="Install or verify OS packages, Rust, Node.js and frontend dependencies.",
        formatter_class=TauriHelpFormatter,
    )
    install_parser.add_argument("--dry-run", action="store_true", help="show actions without running installers")
    install_parser.add_argument("--skip-system-deps", action="store_true", help="skip operating-system packages")
    install_parser.add_argument("--skip-rust", action="store_true", help="skip Rust toolchain preparation")
    install_parser.add_argument("--skip-node", action="store_true", help="skip Node.js/npm checks")
    install_parser.add_argument("--skip-frontend", action="store_true", help="skip frontend dependency installation")
    install_parser.epilog = (
        "examples:\n"
        "  python tools/control.py tauri install --dry-run\n"
        "  python tools/control.py tauri install --skip-system-deps"
    )

    install_appimage_parser = tauri_subparsers.add_parser(
        "install-appimage",
        help="install the latest built AppImage locally",
        description="Copy the newest AppImage, icon and desktop entry into the current user's environment.",
        formatter_class=TauriHelpFormatter,
    )
    install_appimage_parser.add_argument("--dry-run", action="store_true", help="show target files without writing them")
    install_appimage_parser.epilog = "example:\n  python tools/control.py tauri install-appimage --dry-run"

    run_parser = tauri_subparsers.add_parser(
        "run",
        help="start Tauri development mode",
        description="Start Tauri in the background by default and follow its log. Ctrl+C stops it.",
        formatter_class=TauriHelpFormatter,
    )
    run_parser.add_argument("--detach", action="store_true", help="compatibility flag; background is already the default")
    run_parser.add_argument("--foreground", action="store_true", help="run directly in the current terminal")
    run_parser.add_argument("--no-follow", action="store_true", help="return after background start without following logs")
    run_parser.add_argument("--frontend-host", help="override FRONTEND_HOST")
    run_parser.add_argument("--frontend-port", type=int, help="override FRONTEND_PORT")
    run_parser.epilog = (
        "examples:\n"
        "  python tools/control.py tauri run --foreground\n"
        "  python tools/control.py tauri run --no-follow"
    )

    build_parser = tauri_subparsers.add_parser(
        "build",
        help="build desktop artifacts",
        description="Create native Tauri packages for the selected platform strategy.",
        formatter_class=TauriHelpFormatter,
    )
    configure_build_parser(build_parser)
    build_parser.epilog = (
        "examples:\n"
        "  python tools/control.py tauri build --dry-run\n"
        "  python tools/control.py tauri build --target linux --bundles deb,rpm\n"
        "  python tools/control.py tauri build --target windows-portable"
    )

    test_parser = tauri_subparsers.add_parser(
        "test",
        help="validate the desktop tooling",
        description="Check the committed Tauri structure and optionally include doctor, Cargo, and build-plan checks.",
        formatter_class=TauriHelpFormatter,
    )
    test_parser.add_argument("--doctor", action="store_true", help="include Tauri environment diagnostics")
    test_parser.add_argument("--cargo", action="store_true", help="run cargo check and Rust tests with Cargo.lock")
    test_parser.add_argument("--build-dry-run", action="store_true", help="include a Linux build dry-run")
    test_parser.add_argument("--all", action="store_true", help="include every optional check")
    test_parser.epilog = (
        "examples:\n"
        "  python tools/control.py tauri test\n"
        "  python tools/control.py tauri test --cargo\n"
        "  python tools/control.py tauri test --all"
    )

    copy_parser = tauri_subparsers.add_parser(
        "copy",
        help="collect desktop build artifacts",
        description="Copy existing desktop bundles into .dist/desktop or an explicit target directory.",
        formatter_class=TauriHelpFormatter,
    )
    copy_parser.add_argument("--dry-run", action="store_true", help="show copies without writing files")
    copy_parser.add_argument("--target-dir", help="destination directory (default: .dist/desktop)")
    copy_parser.epilog = (
        "examples:\n"
        "  python tools/control.py tauri copy --dry-run\n"
        "  python tools/control.py tauri copy --target-dir ./release"
    )


def main(args: argparse.Namespace) -> int:
    profile = profile_runtime.active_profile(paths.ROOT)
    tauri_enabled = profile.has_feature("tauri")

    if getattr(args, "tauri_command", None) is None:
        tauri_parser = getattr(args, "tauri_parser", None)
        if tauri_parser is not None:
            tauri_parser.print_help()
            if not tauri_enabled:
                logger.info(f"Tauri feature is disabled by active profile '{profile.profile_id}'.")
            return 0
        logger.info("Use 'python tools/control.py tauri --help' to list Tauri commands.")
        return 0

    if not tauri_enabled:
        command = getattr(args, "tauri_command", None)
        if command in {"doctor", "install", "test"}:
            logger.info(
                f"Tauri {command} skipped because the feature is disabled by active profile "
                f"'{profile.profile_id}'."
            )
            return 0
        logger.fail(f"Tauri feature is disabled by active profile '{profile.profile_id}'.")
        return 1

    handlers = {
        "doctor": doctor.main,
        "install": install.main,
        "run": run.main,
        "build": build.main,
        "install-appimage": installappimage.main,
        "test": test.main,
        "copy": copy.main,
    }
    handler = handlers.get(getattr(args, "tauri_command", None))
    if handler is None:
        logger.fail(f"Unknown Tauri command: {getattr(args, 'tauri_command', None)}")
        return 2
    return handler(args)
