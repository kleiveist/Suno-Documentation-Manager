#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
import traceback
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tools import logger
from tools.inst import build, configuration, console, container, db, docs_index, doctor, install, release, run, run_test, stop
from tools.profiles import cli as profile_cli
from tools.profiles import runtime as profile_runtime
from tools.tauri import build as tauri_build
from tools.tauri import control as tauri_control

Handler = Callable[[argparse.Namespace], int]


class HelpFormatter(argparse.RawDescriptionHelpFormatter):
    def __init__(self, prog: str) -> None:
        super().__init__(prog, max_help_position=30, width=100)


class ControlParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:  # type: ignore[override]
        self.print_help(sys.stderr)
        print(file=sys.stderr)
        logger.fail(f"{self.prog}: {message}", stream=sys.stderr)
        logger.info(f"Next step: {self.prog} --help", stream=sys.stderr)
        self.exit(2)


COMMAND_ALIASES: dict[str, str] = {
    "--doctor": "doctor",
    "--install": "install",
    "--run": "run",
    "--stop": "stop",
    "--test": "test",
}

TAURI_COMMAND_ALIASES: dict[str, str] = {
    "--doctor": "doctor",
    "--install": "install",
    "--run": "run",
    "--build": "build",
    "--install-appimage": "install-appimage",
    "--test": "test",
    "--copy": "copy",
}

ROOT_HELP = """
One entry point for the complete project lifecycle.

Need a derived project from the master template?
  init     Generate a profile-based scaffold in .generated/<profile-id> or --target-dir.

Recommended workflow after returning to the project:
  1. doctor   Check tools, dependencies and occupied ports.
  2. install  Install or repair dependencies for enabled features.
  3. run      Start the enabled local development services.
  4. test     Select and run the relevant quality checks.
  5. build    Choose a web, desktop, or container artifact.

Prefer a menu? Start the optional interactive console:
  python tools/control.py console

Groups with their own command maps:
  python tools/control.py build
  python tools/control.py container
  python tools/control.py config
  python tools/control.py db
  python tools/control.py docs
  python tools/control.py tauri
  python tools/control.py version
  python tools/control.py release
"""

ROOT_EXAMPLES = """
examples:
  python tools/control.py init
  python tools/control.py init --profile web-only --dry-run
  python tools/control.py doctor
  python tools/control.py install
  python tools/control.py run --detach
  python tools/control.py stop
  python tools/control.py test --suite all --report
  python tools/control.py build web
  python tools/control.py build container
  python tools/control.py config doctor
  python tools/control.py db doctor
  python tools/control.py db current
  python tools/control.py docs index --dry-run
  python tools/control.py tauri

Compatibility:
  The former aliases --doctor, --install, --run, --stop, --test and --build remain available.
"""


def _normalize_argv(argv: list[str] | None) -> list[str]:
    normalized = list(sys.argv[1:] if argv is None else argv)
    if not normalized:
        return normalized

    first = normalized[0].lower()
    if first == "--build":
        if len(normalized) >= 2 and normalized[1].lower() == "--desktop":
            return ["build", "desktop", *normalized[2:]]
        return ["build", "web", *normalized[1:]]

    if len(normalized) >= 2 and first == "tauri":
        tauri_alias = TAURI_COMMAND_ALIASES.get(normalized[1].lower())
        if tauri_alias:
            normalized[1] = tauri_alias
            return normalized

    alias = COMMAND_ALIASES.get(first)
    if alias:
        show_test_guide = first == "--test" and len(normalized) == 1
        normalized[0] = alias
        if show_test_guide:
            normalized.append("--suite-help")
    return normalized


def _add_examples(parser: argparse.ArgumentParser, examples: str) -> None:
    parser.epilog = examples.strip()


def _build_parser() -> argparse.ArgumentParser:
    parser = ControlParser(
        prog="python tools/control.py",
        description=ROOT_HELP.strip(),
        epilog=ROOT_EXAMPLES.strip(),
        formatter_class=HelpFormatter,
    )
    subparsers = parser.add_subparsers(
        dest="command",
        title="command map",
        metavar="<command>",
    )

    init_parser = subparsers.add_parser(
        "init",
        help="generate a derived project from a selected profile",
        description="Create a profile-based project scaffold from the master template without modifying this repository.",
        formatter_class=HelpFormatter,
    )
    init_parser.add_argument("--profile", help="profile id to generate; omit to choose interactively")
    init_parser.add_argument(
        "--with",
        dest="optional_features",
        action="append",
        default=[],
        metavar="FEATURE",
        help="add an optional capability; repeat the flag or use comma-separated feature ids",
    )
    init_parser.add_argument("--name", dest="project_name", help="project display name")
    init_parser.add_argument("--slug", dest="project_slug", help="lowercase package/application slug")
    init_parser.add_argument("--identifier", help="Tauri reverse-domain identifier")
    init_parser.add_argument(
        "--target-dir",
        metavar="PATH",
        help="destination directory (default: .generated/<profile-id>[-<capability>] below the template root)",
    )
    init_parser.add_argument("--dry-run", action="store_true", help="show the scaffold plan without writing files")
    _add_examples(
        init_parser,
        """examples:
  python tools/control.py init
  python tools/control.py init --profile web-only
  python tools/control.py init --profile web-cloud --with postgres
  python tools/control.py init --profile desktop-cloud --target-dir ../desktop-cloud-app
  python tools/control.py init --profile desktop-cloud --name CustomerApp --identifier com.customer.app
  python tools/control.py init --profile full-platform --dry-run""",
    )

    doctor_parser = subparsers.add_parser(
        "doctor",
        help="inspect the development environment",
        description="Inspect runtimes, dependencies, project files and local ports without changing them.",
        formatter_class=HelpFormatter,
    )
    doctor_parser.add_argument("--watch", action="store_true", help="repeat checks until interrupted")
    doctor_parser.add_argument("--interval", type=int, default=5, help="seconds between watch checks (default: 5)")
    _add_examples(
        doctor_parser,
        """examples:
  python tools/control.py doctor
  python tools/control.py doctor --watch --interval 10""",
    )

    install_parser = subparsers.add_parser(
        "install",
        help="install or repair project dependencies",
        description="Prepare the local frontend, backend and optional E2E environment.",
        formatter_class=HelpFormatter,
    )
    install_parser.add_argument("--skip-frontend", action="store_true", help="do not run npm install")
    install_parser.add_argument("--skip-backend", action="store_true", help="do not prepare the Python venv")
    install_parser.add_argument("--skip-tooling", action="store_true", help="do not prepare the shared tooling test venv")
    install_parser.add_argument("--skip-playwright", action="store_true", help="do not install Playwright Chromium")
    _add_examples(
        install_parser,
        """examples:
  python tools/control.py install
  python tools/control.py install --skip-playwright
  python tools/control.py install --skip-frontend""",
    )

    console_parser = subparsers.add_parser(
        "console",
        help="open an interactive menu for common tasks",
        description="Open a numbered menu that guides you through common project actions.",
        formatter_class=HelpFormatter,
    )
    _add_examples(
        console_parser,
        """sections:
  environment  Doctor and dependency setup
  services     Foreground/background start and stop
  tests        Quick, complete, individual and reported test runs
  builds       Web packages and guided desktop targets
  Tauri        Diagnostics, setup, development and artifacts
  docs         PyGitIndex preview and update

example:
  python tools/control.py console""",
    )

    build_parser = subparsers.add_parser(
        "build",
        help="choose a web, desktop, or container artifact",
        description="Build map. Choose one target; a bare 'build' only shows this guide.",
        formatter_class=HelpFormatter,
    )
    build_parser.set_defaults(build_parser=build_parser)
    build_subparsers = build_parser.add_subparsers(
        dest="build_command",
        title="build targets",
        metavar="<target>",
    )
    web_parser = build_subparsers.add_parser(
        "web",
        help="compile and package the Vite web app",
        description="Build frontend/dist and create .dist/web/suno-documentation-manager-web.zip.",
        formatter_class=HelpFormatter,
    )
    _add_examples(web_parser, "examples:\n  python tools/control.py build web")
    desktop_parser = build_subparsers.add_parser(
        "desktop",
        help="build Tauri desktop artifacts",
        description="Build native desktop artifacts through the restored Tauri tooling.",
        formatter_class=HelpFormatter,
    )
    tauri_control.configure_build_parser(desktop_parser)
    _add_examples(
        desktop_parser,
        """examples:
  python tools/control.py build desktop --dry-run
  python tools/control.py build desktop --target linux --bundles deb,rpm
  python tools/control.py build desktop --target windows-portable""",
    )
    container_build_parser = build_subparsers.add_parser(
        "container",
        help="build profile-aware backend and frontend images",
        description="Build provider-neutral production images for a cloud-enabled profile.",
        formatter_class=HelpFormatter,
    )
    container_build_parser.add_argument(
        "--component",
        choices=["all", "backend", "frontend"],
        default="all",
        help="image component to build (default: all)",
    )
    container_build_parser.add_argument("--no-cache", action="store_true", help="disable Docker build cache")
    _add_examples(
        build_parser,
        """examples:
  python tools/control.py build web
  python tools/control.py build desktop --dry-run
  python tools/control.py build container

More desktop commands:
  python tools/control.py tauri""",
    )

    db_parser = subparsers.add_parser(
        "db",
        help="manage optional database diagnostics and migrations",
        description="Database command map. Commands require the active project to enable the database feature.",
        formatter_class=HelpFormatter,
    )

    container_parser = subparsers.add_parser(
        "container",
        help="inspect and validate provider-neutral container deployment files",
        description="Container command map for Docker and Compose validation.",
        formatter_class=HelpFormatter,
    )
    container_parser.set_defaults(container_parser=container_parser)
    container_subparsers = container_parser.add_subparsers(dest="container_command", metavar="<action>")
    container_subparsers.add_parser("doctor", help="check Docker, Compose, and deployment files")
    container_subparsers.add_parser("validate", help="validate the Compose model without starting services")
    _add_examples(
        container_parser,
        "examples:\n  python tools/control.py container doctor\n  python tools/control.py container validate",
    )

    version_parser = subparsers.add_parser(
        "version",
        help="show or validate the application version",
        description="Read the VERSION source of truth and compare published metadata.",
        formatter_class=HelpFormatter,
    )
    version_subparsers = version_parser.add_subparsers(dest="version_command", metavar="<action>")
    version_subparsers.add_parser("check", help="verify all enabled component versions")
    version_subparsers.add_parser("sync", help="copy VERSION into enabled component metadata")

    release_parser = subparsers.add_parser(
        "release",
        help="run non-publishing release validation",
        description="Validate identity, version, security, and repository state without publishing.",
        formatter_class=HelpFormatter,
    )
    release_parser.set_defaults(release_parser=release_parser)
    release_subparsers = release_parser.add_subparsers(dest="release_command", metavar="<action>")
    release_subparsers.add_parser("check", help="run the production release gate")
    db.configure_parser(db_parser)
    _add_examples(
        db_parser,
        """examples:
  python tools/control.py db doctor
  python tools/control.py db doctor --connect
  python tools/control.py db current
  python tools/control.py db upgrade
  python tools/control.py db downgrade
  python tools/control.py db revision --message 'add widgets'""",
    )

    config_parser = subparsers.add_parser(
        "config",
        help="show and validate effective runtime configuration",
        description="Configuration command map. Values resolve from CLI, process environment, .env, and defaults.",
        formatter_class=HelpFormatter,
    )
    configuration.configure_parser(config_parser)
    _add_examples(
        config_parser,
        """examples:
  python tools/control.py config show
  python tools/control.py config doctor
  python tools/control.py config show --backend-port 9000""",
    )

    docs_parser = subparsers.add_parser(
        "docs",
        help="manage documentation navigation with PyGitIndex",
        description="Documentation map. A bare 'docs' shows the available maintenance actions.",
        formatter_class=HelpFormatter,
    )
    docs_parser.set_defaults(docs_parser=docs_parser)
    docs_subparsers = docs_parser.add_subparsers(
        dest="docs_command",
        title="documentation actions",
        metavar="<action>",
    )
    docs_index_parser = docs_subparsers.add_parser(
        "index",
        help="regenerate indices and backlinks with the system PyGitIndex script",
        description="Run PyGitIndex and keep its generated navigation labels in English.",
        formatter_class=HelpFormatter,
    )
    docs_index_parser.add_argument("--dry-run", action="store_true", help="preview PyGitIndex without writing files")
    docs_index_parser.add_argument("--force", action="store_true", help="replace index files instead of updating markers")
    docs_index_parser.add_argument("--compact", action="store_true", help="list only directory overviews in README")
    docs_index_parser.add_argument("--no-backlinks", action="store_true", help="do not add or update Markdown backlinks")
    docs_index_parser.add_argument("--no-readme", action="store_true", help="do not update the README navigation block")
    docs_index_parser.add_argument(
        "--script",
        metavar="PATH",
        help="explicit PyGitIndex.py path (otherwise use PYGITINDEX_PATH, PATH or known user locations)",
    )
    docs_index_parser.add_argument("--docs-dir", default="docs", help="documentation directory (default: docs)")
    _add_examples(
        docs_index_parser,
        """examples:
  python tools/control.py docs index --dry-run
  python tools/control.py docs index
  python tools/control.py docs index --compact
  python tools/control.py docs index --script /path/to/PyGitIndex.py""",
    )
    _add_examples(
        docs_parser,
        """examples:
  python tools/control.py docs index --dry-run
  python tools/control.py docs index""",
    )

    run_parser = subparsers.add_parser(
        "run",
        help="start enabled development services",
        description="Start the services enabled by project-profile.toml. Foreground is the default; Ctrl+C stops them.",
        formatter_class=HelpFormatter,
    )
    run_parser.add_argument("--frontend-host", help="override FRONTEND_HOST")
    run_parser.add_argument("--frontend-port", type=int, help="override FRONTEND_PORT")
    run_parser.add_argument("--backend-host", help="override BACKEND_HOST")
    run_parser.add_argument("--backend-port", type=int, help="override BACKEND_PORT")
    run_parser.add_argument("--detach", action="store_true", help="run in background and write logs to tools/.runtime")
    _add_examples(
        run_parser,
        """examples:
  python tools/control.py run
  python tools/control.py run --detach
  python tools/control.py stop""",
    )

    stop_parser = subparsers.add_parser(
        "stop",
        help="stop tracked development services",
        description="Stop services recorded by detached runs and optionally clean stale project ports.",
        formatter_class=HelpFormatter,
    )
    stop_parser.add_argument("--frontend-port", type=int, help="override FRONTEND_PORT for stale-listener cleanup")
    stop_parser.add_argument("--backend-port", type=int, help="override BACKEND_PORT for stale-listener cleanup")
    stop_parser.add_argument("--tracked-only", action="store_true", help="do not inspect stale listeners")
    _add_examples(stop_parser, "examples:\n  python tools/control.py stop\n  python tools/control.py stop --tracked-only")

    test_parser = subparsers.add_parser(
        "test",
        help="select test suites and optional reports",
        description="Test map. A bare 'test' shows this guide and does not run every suite unexpectedly.",
        formatter_class=HelpFormatter,
    )
    test_parser.set_defaults(test_parser=test_parser)
    test_parser.add_argument(
        "--suite",
        choices=["api", "schema", "database", "postgres", "frontend", "e2e", "tools", "tauri", "all"],
        default=None,
        help="suite to run; use all for the complete configured set",
    )
    test_parser.add_argument("--no-start", action="store_true", help="do not start services for E2E tests")
    test_parser.add_argument(
        "--report",
        nargs="?",
        const="md",
        choices=["md", "markdown", "json", "all", "done"],
        help="write a report, or use '--report done' to remove .report",
    )
    test_parser.add_argument("--suite-help", action="store_true", help=argparse.SUPPRESS)
    _add_examples(
        test_parser,
        """suites:
  api       FastAPI tests
  schema    shared JSON Schema examples
  database  SQLAlchemy and session unit tests
  postgres  PostgreSQL integration tests (skipped without DATABASE_URL_TEST)
  frontend  Vitest tests
  e2e       Playwright tests (skipped until configured)
  tools     restored Python tooling tests
  tauri     Tauri structure, cargo check, and Rust tests
  all       every configured suite

examples:
  python tools/control.py test --suite tools
  python tools/control.py test --suite all --report
  python tools/control.py test --report done""",
    )

    tauri_parser = subparsers.add_parser(
        "tauri",
        help="open the Tauri desktop command map",
        description="Tauri-specific diagnostics, setup, development, builds and artifact handling.",
        formatter_class=HelpFormatter,
    )
    tauri_control.configure_parser(tauri_parser)

    return parser


def _handle_build(args: argparse.Namespace) -> int:
    if getattr(args, "build_command", None) is None:
        args.build_parser.print_help()
        return 0
    if args.build_command == "web":
        return build.main(args)
    if args.build_command == "desktop":
        if not profile_runtime.feature_enabled("tauri", ROOT):
            profile = profile_runtime.active_profile(ROOT)
            logger.fail(f"Tauri desktop build is disabled by active profile '{profile.profile_id}'.")
            return 1
        return tauri_build.main(args)
    if args.build_command == "container":
        return container.build(args)
    logger.fail(f"Unknown build target: {args.build_command}")
    return 2


def _handle_test(args: argparse.Namespace) -> int:
    if args.report == "done":
        return run_test.main(args)
    if args.suite is None and not args.suite_help:
        args.test_parser.print_help()
        return 0
    if args.suite is None:
        args.suite = "all"
    return run_test.main(args)


def _handle_console(_args: argparse.Namespace) -> int:
    return console.main()


def _handle_docs(args: argparse.Namespace) -> int:
    if getattr(args, "docs_command", None) is None:
        args.docs_parser.print_help()
        return 0
    if args.docs_command == "index":
        return docs_index.main(args)
    logger.fail(f"Unknown documentation action: {args.docs_command}")
    return 2


def _handle_container(args: argparse.Namespace) -> int:
    command = getattr(args, "container_command", None)
    if command is None:
        args.container_parser.print_help()
        return 0
    if command == "doctor":
        return container.doctor(args)
    if command == "validate":
        return container.validate(args)
    logger.fail(f"Unknown container action: {command}")
    return 2


def _handlers() -> dict[str, Handler]:
    return {
        "init": profile_cli.main,
        "doctor": doctor.main,
        "install": install.main,
        "console": _handle_console,
        "build": _handle_build,
        "container": _handle_container,
        "config": configuration.main,
        "db": db.main,
        "docs": _handle_docs,
        "version": release.version,
        "release": release.release,
        "run": run.run_command,
        "stop": stop.main,
        "test": _handle_test,
        "tauri": tauri_control.main,
    }


def main(argv: list[str] | None = None) -> int:
    normalized_argv = _normalize_argv(argv)
    parser = _build_parser()
    args = parser.parse_args(normalized_argv)
    if args.command is None:
        parser.print_help()
        return 0

    args.display_argv = list(sys.argv[1:] if argv is None else argv)
    handler = _handlers().get(args.command)
    if handler is None:
        logger.fail(f"Unknown command: {args.command}")
        logger.info(f"Next step: {parser.prog} --help")
        return 2

    try:
        code = handler(args)
        return 0 if code is None else int(code)
    except KeyboardInterrupt:
        logger.warn("Interrupted by user")
        return 130
    except Exception as exc:  # pragma: no cover
        logger.fail(f"Unhandled error: {exc}")
        for line in traceback.format_exc().strip().splitlines():
            logger.info(line)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
