from __future__ import annotations

import hashlib
import json
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[2]
POLYFORM_SHIELD_SHA256 = "67530f8e9adfcc5d2e9d72b804500cebb7472ff84c34a6729a80a2a9be901ee6"
LGPL_2_1_SHA256 = "20e50fe7aae3e56378ebf0417d9de904f55a0e61e4df315333e632a4d3555d95"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_polyform_shield_body_and_required_notice_are_byte_stable() -> None:
    assert _sha256(ROOT / "LICENSE") == POLYFORM_SHIELD_SHA256
    assert (ROOT / "NOTICE").read_text(encoding="utf-8").splitlines() == [
        "Required Notice: Copyright (c) 2026 Blobbite.com",
        "Required Notice: Licensor: Blobbite.com",
    ]
    assert "Licensor Line of Business:" not in (ROOT / "NOTICE").read_text(encoding="utf-8")


def test_third_party_notices_do_not_relicense_components() -> None:
    notices = (ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")

    assert "does not replace or relicense third-party components" in notices
    assert "Distribution of these sidecars therefore remains **NOT READY**" in notices
    assert "Required Notice:" not in notices
    assert _sha256(ROOT / "src-tauri" / "binaries" / "LGPL-2.1.txt") == LGPL_2_1_SHA256


def test_cargo_and_tauri_ship_the_product_and_third_party_license_boundary() -> None:
    cargo = tomllib.loads((ROOT / "src-tauri" / "Cargo.toml").read_text(encoding="utf-8"))
    tauri = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    bundle = tauri["bundle"]

    assert cargo["package"]["license-file"] == "../LICENSE"
    assert "license" not in cargo["package"]
    assert tauri["productName"] == "sunodm"
    assert tauri["version"] == "0.1.0"
    assert tauri["identifier"] == "com.grav0id.sunodoc"
    assert tauri["mainBinaryName"] == "sunodm"
    assert bundle["externalBin"] == ["binaries/fpcalc"]
    assert bundle["copyright"] == "Copyright (c) 2026 Blobbite.com"
    assert bundle["licenseFile"] == "../LICENSE"
    assert bundle["resources"] == {
        "../LICENSE": "LICENSE",
        "../NOTICE": "NOTICE",
        "../THIRD_PARTY_NOTICES.md": "THIRD_PARTY_NOTICES.md",
        "binaries/CHROMAPRINT-1.6.1-PROVENANCE.md": "third-party/CHROMAPRINT-1.6.1-PROVENANCE.md",
        "binaries/CHROMAPRINT-LICENSE.md": "third-party/CHROMAPRINT-LICENSE.md",
        "binaries/LGPL-2.1.txt": "third-party/LGPL-2.1.txt",
        "assets/fonts/LICENSE-DejaVu.txt": "third-party/LICENSE-DejaVu.txt",
        "vendor/sigstore-tsa/LICENSE": "third-party/sigstore-tsa/LICENSE",
        "vendor/sigstore-tsa/README-SUNODM.md": "third-party/sigstore-tsa/README-SUNODM.md",
    }
