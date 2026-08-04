#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import plistlib
import shutil
import stat
import sys
from pathlib import Path
from typing import Any

from project_metadata import require_project


def build_info_plist(root: Path) -> dict[str, Any]:
    metadata = require_project(root)
    info: dict[str, Any] = {
        "CFBundleDevelopmentRegion": "English",
        "CFBundleDisplayName": metadata.product_name,
        "CFBundleExecutable": metadata.binary_name,
        "CFBundleIconFile": "icon.icns",
        "CFBundleIdentifier": metadata.identifier,
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": metadata.product_name,
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": metadata.version,
        "CFBundleVersion": metadata.version,
        "LSMinimumSystemVersion": metadata.minimum_system_version,
        "LSRequiresCarbon": True,
        "NSHighResolutionCapable": True,
        "CFBundleURLTypes": [
            {
                "CFBundleTypeRole": "Editor",
                "CFBundleURLName": f"{metadata.identifier}.{scheme}",
                "CFBundleURLSchemes": [scheme],
            }
            for scheme in metadata.deep_link_schemes
        ],
    }

    custom_plist = root / "src-tauri" / "Info.plist"
    if custom_plist.is_file():
        with custom_plist.open("rb") as stream:
            custom = plistlib.load(stream)
        if not isinstance(custom, dict):
            raise ValueError(f"custom plist must contain a dictionary: {custom_plist}")
        info.update(custom)
    return info


def assemble_app(binary: Path, root: Path, output_dir: Path) -> Path:
    metadata = require_project(root)
    binary = binary.expanduser().resolve()
    if not binary.is_file():
        raise FileNotFoundError(f"Mach-O binary not found: {binary}")

    output_dir.mkdir(parents=True, exist_ok=True)
    app_path = output_dir / f"{metadata.product_name}.app"
    if app_path.exists():
        shutil.rmtree(app_path)

    contents = app_path / "Contents"
    macos_dir = contents / "MacOS"
    resources_dir = contents / "Resources"
    macos_dir.mkdir(parents=True)
    resources_dir.mkdir(parents=True)

    executable = macos_dir / metadata.binary_name
    shutil.copy2(binary, executable)
    executable.chmod(
        executable.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    )
    shutil.copy2(metadata.icon_path, resources_dir / "icon.icns")

    with (contents / "Info.plist").open("wb") as stream:
        plistlib.dump(build_info_plist(root), stream, fmt=plistlib.FMT_XML, sort_keys=True)

    os.chmod(contents / "Info.plist", 0o644)
    os.chmod(resources_dir / "icon.icns", 0o644)
    return app_path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Assemble FyAgent.app from a cross-compiled Universal Mach-O executable."
    )
    parser.add_argument("binary", help="Universal Mach-O executable")
    parser.add_argument("--project-root", default=".", help="FyAgent project root")
    parser.add_argument("--output-dir", required=True, help="temporary app output directory")
    args = parser.parse_args()

    try:
        app_path = assemble_app(
            Path(args.binary),
            Path(args.project_root).expanduser().resolve(),
            Path(args.output_dir).expanduser().resolve(),
        )
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(app_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
