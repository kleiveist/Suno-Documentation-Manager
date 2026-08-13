from __future__ import annotations

import argparse

from tools import logger
from tools.tauri import common, paths
from tools.tauri.build import appimage

DEFAULT_LINUX_BUNDLES = "deb,rpm,appimage"


def main(args: argparse.Namespace) -> int:
    if not getattr(args, "dry_run", False) and not common.frontend_dependencies_ready():
        missing = ", ".join(str(path.relative_to(paths.ROOT)) for path in common.missing_frontend_dependency_paths())
        logger.fail(
            "Frontend dependencies are incomplete. "
            f"Missing: {missing}. "
            "Run 'python tools/control.py tauri install --skip-system-deps --skip-rust' first."
        )
        return 1

    dry_run = bool(getattr(args, "dry_run", False))
    bundles = getattr(args, "bundles", None) or DEFAULT_LINUX_BUNDLES
    if _bundles_include_appimage(bundles) and not dry_run:
        if getattr(args, "skip_appimage_preflight", False):
            logger.warn("Skipping AppImage prerequisite checks; linuxdeploy may still fail.")
        elif not appimage._appimage_prerequisites_ready("python tools/control.py tauri build --target linux"):
            return 1

    command = common.tauri_cli_command("build", "--bundles", bundles)
    common.print_build_plan("linux", command, dry_run=dry_run, bundles=bundles)
    env = appimage.appimage_build_env() if _bundles_include_appimage(bundles) else None
    appimage_snapshot = appimage._appimage_snapshot() if _bundles_include_appimage(bundles) else {}
    result = common.run_command(command, cwd=paths.ROOT, dry_run=dry_run, env=env)
    if result.returncode != 0 and _bundles_include_appimage(bundles) and not dry_run:
        if appimage.is_linuxdeploy_failure(result):
            fresh_appimage = appimage._fresh_appimage_from_snapshot(appimage_snapshot)
            if fresh_appimage is not None:
                logger.warn(
                    "Tauri linuxdeploy failed, but it produced "
                    f"{fresh_appimage.name}; continuing with that AppImage."
                )
                logger.ok("Linux Tauri build completed")
                common.print_build_artifacts()
                return 0
            fallback_code = appimage.package_existing_appdir()
            if fallback_code == 0:
                logger.warn("Tauri linuxdeploy failed, but AppImage was packaged from the generated AppDir.")
                logger.ok("Linux Tauri build completed")
                common.print_build_artifacts()
                return 0
        else:
            logger.fail(
                "AppImage fallback skipped: Tauri failed before linuxdeploy. "
                "Fix the build error above; stale AppImage artifacts were not treated as a successful build."
            )

    code = common.print_result(result, "Linux Tauri build completed", "Linux Tauri build failed")
    if code == 0 and dry_run:
        logger.info("📁 Dry-run finished; no new artifacts were created.")
    elif code == 0:
        common.print_build_artifacts()
    return code


def _bundles_include_appimage(bundles: str) -> bool:
    return any(item.strip().lower() == "appimage" for item in bundles.split(","))
