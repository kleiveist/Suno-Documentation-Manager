from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import pytest

from tools.profiles import generator, loader

ROOT = Path(__file__).resolve().parents[2]
PROFILES_DIR = ROOT / "profiles"
ISSUE_FORMS = tuple(sorted((ROOT / ".github" / "ISSUE_TEMPLATE").glob("[0-9][0-9]-*.yml")))
ISSUE_CONFIG = ROOT / ".github" / "ISSUE_TEMPLATE" / "config.yml"
PULL_REQUEST_TEMPLATE = ROOT / ".github" / "pull_request_template.md"
REPOSITORY_URL = "https://github.com/kleiveist/Suno-Documentation-Manager"
MASTER_COMMUNITY_TEMPLATES = (
    *ISSUE_FORMS,
    ISSUE_CONFIG,
    PULL_REQUEST_TEMPLATE,
    ROOT / "CODE_OF_CONDUCT.md",
    ROOT / "CONTRIBUTING.md",
    ROOT / ".github" / "SECURITY.md",
)
MASTER_COMMUNITY_PRESENT = any(path.exists() for path in MASTER_COMMUNITY_TEMPLATES)
MASTER_ONLY_SKIP_REASON = "template community forms are intentionally absent from the SunoDM product"
MASTER_ONLY_PATHS = (
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    ".github/CODEOWNERS",
    ".github/SECURITY.md",
    ".github/dependabot.yml",
    ".github/ISSUE_TEMPLATE",
    ".github/pull_request_template.md",
    ".github/workflows",
)
REQUIRED_NOTICE_LINES = (
    "Required Notice: Copyright (c) 2026 Blobbite.com",
    "Required Notice: Licensor: Blobbite.com",
)


def _scaffold_web_only(target: Path) -> None:
    catalog = loader.load_catalog(PROFILES_DIR, validate_paths=False)
    plan = generator.build_scaffold_plan(
        catalog,
        project_root=ROOT,
        target_dir=target,
        profile_id="web-only",
    )

    generator.scaffold_project(plan)


def test_sunodm_owns_polyform_shield_license_notice_and_product_governance(
    tmp_path: Path,
) -> None:
    target = tmp_path / "web-only"
    _scaffold_web_only(target)

    license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
    assert (target / "LICENSE").read_text(encoding="utf-8") == license_text
    assert "# PolyForm Shield License 1.0.0" in license_text
    assert "https://polyformproject.org/licenses/shield/1.0.0" in license_text
    notice_text = (ROOT / "NOTICE").read_text(encoding="utf-8")
    assert tuple(notice_text.splitlines()) == REQUIRED_NOTICE_LINES
    assert (target / "NOTICE").read_text(encoding="utf-8") == notice_text
    third_party_notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")
    assert (target / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8") == third_party_notices
    assert all(not (target / relative).exists() for relative in MASTER_ONLY_PATHS)
    generated_readme = (target / "README.md").read_text(encoding="utf-8")
    assert "CODE_OF_CONDUCT.md" not in generated_readme
    assert "CONTRIBUTING.md" not in generated_readme
    assert ".github/SECURITY.md" not in generated_readme
    assert "[PolyForm Shield License 1.0.0](LICENSE)" in generated_readme
    assert "[`NOTICE`](NOTICE)" in generated_readme


@pytest.mark.skipif(
    not MASTER_COMMUNITY_PRESENT,
    reason=MASTER_ONLY_SKIP_REASON,
)
def test_master_community_template_links_use_rendered_github_context() -> None:
    templates = (*ISSUE_FORMS, PULL_REQUEST_TEMPLATE)
    for template in templates:
        targets = re.findall(r"\[[^\]]+\]\(([^)]+)\)", template.read_text(encoding="utf-8"))
        for target in targets:
            assert target.startswith(f"{REPOSITORY_URL}/"), (
                "Community template links render in issue or pull-request bodies and must use a stable "
                f"repository URL: {template}: {target}"
            )

    security_policy_url = f"{REPOSITORY_URL}/security/policy"
    assert all(security_policy_url in form.read_text(encoding="utf-8") for form in ISSUE_FORMS)
    assert security_policy_url in PULL_REQUEST_TEMPLATE.read_text(encoding="utf-8")
    assert f"{REPOSITORY_URL}/security" in ISSUE_CONFIG.read_text(encoding="utf-8")


@pytest.mark.skipif(not MASTER_COMMUNITY_PRESENT, reason=MASTER_ONLY_SKIP_REASON)
def test_generated_product_skips_master_only_link_validation(tmp_path: Path) -> None:
    target = tmp_path / "web-only"
    _scaffold_web_only(target)
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "-q",
            "tools/tests/test_community_ownership.py::test_master_community_template_links_use_rendered_github_context",
        ],
        cwd=target,
        text=True,
        capture_output=True,
        check=False,
    )

    assert completed.returncode == 0, completed.stdout + completed.stderr
    assert "1 skipped" in completed.stdout
