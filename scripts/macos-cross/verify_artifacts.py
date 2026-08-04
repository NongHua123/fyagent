#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import plistlib
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence

from project_metadata import ProjectMetadata, require_project


BAD_MACHO_MARKERS = (
    "/home/",
    "/mnt/",
    "/lib/x86_64-linux-gnu",
    "/usr/lib/x86_64-linux-gnu",
    ".so",
)
EXPECTED_ARCHITECTURES = {"arm64", "x86_64"}
UDIF_MAGIC_FILE = Path(__file__).with_name("udif.magic")


def run(arguments: Sequence[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(arguments),
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verify_adhoc_signature(path: Path, label: str) -> list[str]:
    signature = run(["rcodesign", "print-signature-info", str(path)], check=False)
    print(signature.stdout.rstrip())
    if signature.returncode != 0 or not signature.stdout.strip():
        return [f"rcodesign could not verify the ad-hoc {label} signature"]
    if "CodeSignatureFlags(ADHOC)" not in signature.stdout:
        return [f"{label} signature is not marked ad-hoc"]
    if not re.search(r"^\s*cms:\s+null\s*$", signature.stdout, re.MULTILINE):
        return [f"{label} signature unexpectedly contains a CMS identity"]
    return []


def verify_universal_binary(binary: Path, deployment_target: str) -> list[str]:
    errors: list[str] = []
    if not binary.is_file():
        return [f"Mach-O binary not found: {binary}"]
    if not os.access(binary, os.X_OK):
        errors.append(f"Mach-O binary is not executable: {binary}")

    file_result = run(["file", str(binary)], check=False)
    print(file_result.stdout.rstrip())
    if file_result.returncode != 0 or "Mach-O universal binary" not in file_result.stdout:
        errors.append("main executable is not recognized as a Mach-O universal binary")

    lipo_result = run(["lipo", "-archs", str(binary)], check=False)
    print(lipo_result.stdout.rstrip())
    architectures = set(lipo_result.stdout.split()) if lipo_result.returncode == 0 else set()
    if architectures != EXPECTED_ARCHITECTURES:
        errors.append(
            f"Universal executable must contain exactly arm64 and x86_64; found {sorted(architectures)}"
        )

    otool_libraries = run(["xcrun", "otool", "-L", str(binary)], check=False)
    print(otool_libraries.stdout.rstrip())
    if otool_libraries.returncode != 0:
        errors.append("otool could not inspect Mach-O load commands")
    if any(marker in otool_libraries.stdout for marker in BAD_MACHO_MARKERS):
        errors.append("Linux/WSL dependency contamination appears in Mach-O load commands")

    deployment_versions: dict[str, str] = {}
    missing_deployment_metadata: list[str] = []
    for architecture in sorted(EXPECTED_ARCHITECTURES):
        otool_load_commands = run(
            ["xcrun", "otool", "-arch", architecture, "-l", str(binary)],
            check=False,
        )
        minimum_versions = re.findall(
            r"^\s*minos\s+([0-9.]+)\s*$",
            otool_load_commands.stdout,
            re.MULTILINE,
        )
        if otool_load_commands.returncode != 0 or len(minimum_versions) != 1:
            missing_deployment_metadata.append(architecture)
            continue
        deployment_versions[architecture] = minimum_versions[0]

    if missing_deployment_metadata:
        errors.append(
            "could not find deployment metadata for both Universal slices; "
            f"missing or ambiguous: {', '.join(missing_deployment_metadata)}"
        )
    mismatched_versions = {
        architecture: version
        for architecture, version in deployment_versions.items()
        if version != deployment_target
    }
    if mismatched_versions:
        errors.append(
            f"Mach-O deployment target must be {deployment_target}; "
            f"found {mismatched_versions}"
        )
    return errors


def _deep_link_schemes(info: dict[str, Any]) -> set[str]:
    schemes: set[str] = set()
    url_types = info.get("CFBundleURLTypes", [])
    if not isinstance(url_types, list):
        return schemes
    for item in url_types:
        if not isinstance(item, dict):
            continue
        values = item.get("CFBundleURLSchemes", [])
        if isinstance(values, list):
            schemes.update(str(value) for value in values)
    return schemes


def verify_app(metadata: ProjectMetadata, app: Path, deployment_target: str) -> list[str]:
    errors: list[str] = []
    if not app.is_dir():
        return [f"app bundle not found: {app}"]
    if app.name != f"{metadata.product_name}.app":
        errors.append(f"unexpected app name: {app.name}")

    info_path = app / "Contents" / "Info.plist"
    if not info_path.is_file():
        return errors + [f"Info.plist is missing: {info_path}"]
    with info_path.open("rb") as stream:
        info = plistlib.load(stream)
    expected_info = {
        "CFBundleDisplayName": metadata.product_name,
        "CFBundleExecutable": metadata.binary_name,
        "CFBundleIdentifier": metadata.identifier,
        "CFBundleShortVersionString": metadata.version,
        "LSMinimumSystemVersion": metadata.minimum_system_version,
    }
    for key, expected in expected_info.items():
        if info.get(key) != expected:
            errors.append(f"Info.plist {key} must be {expected!r}; found {info.get(key)!r}")
    if _deep_link_schemes(info) != set(metadata.deep_link_schemes):
        errors.append("Info.plist deep-link schemes do not match the project contract")

    icon = app / "Contents" / "Resources" / "icon.icns"
    if not icon.is_file():
        errors.append(f"app icon is missing: {icon}")
    executable = app / "Contents" / "MacOS" / metadata.binary_name
    errors.extend(verify_universal_binary(executable, deployment_target))

    errors.extend(verify_adhoc_signature(app, "app"))
    return errors


def verify_dmg(dmg: Path) -> list[str]:
    errors: list[str] = []
    if not dmg.is_file():
        return [f"DMG not found: {dmg}"]
    if dmg.stat().st_size < 512:
        return ["DMG is smaller than a UDIF trailer"]
    with dmg.open("rb") as stream:
        stream.seek(-512, os.SEEK_END)
        trailer = stream.read(512)
    if trailer[:4] != b"koly":
        errors.append("UDIF koly trailer is missing")
    else:
        print("UDIF trailer: koly")

    file_result = run(
        ["file", "--brief", "--magic-file", str(UDIF_MAGIC_FILE), str(dmg)],
        check=False,
    )
    print(file_result.stdout.rstrip())
    if file_result.returncode != 0 or "Apple Disk Image" not in file_result.stdout:
        errors.append("file does not recognize the UDIF trailer as an Apple Disk Image")

    errors.extend(verify_adhoc_signature(dmg, "DMG"))
    return errors


def verify_manifest(manifest_path: Path, dmg: Path) -> list[str]:
    if not manifest_path.is_file():
        return [f"manifest not found: {manifest_path}"]
    if not dmg.is_file():
        return [f"DMG not found: {dmg}"]
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot parse manifest: {error}"]
    if not isinstance(manifest, dict):
        return ["manifest root must be an object"]

    errors: list[str] = []
    artifact = manifest.get("artifact")
    target = manifest.get("target")
    validation = manifest.get("validation")
    if manifest.get("schemaVersion") != 1:
        errors.append("manifest schemaVersion must be 1")
    if not isinstance(artifact, dict):
        errors.append("manifest artifact must be an object")
    else:
        expected_artifact = {
            "fileName": dmg.name,
            "sizeBytes": dmg.stat().st_size,
            "sha256": sha256_file(dmg),
        }
        for key, expected in expected_artifact.items():
            if artifact.get(key) != expected:
                errors.append(
                    f"manifest artifact.{key} must be {expected!r}; "
                    f"found {artifact.get(key)!r}"
                )
    if not isinstance(target, dict) or set(target.get("architectures", [])) != EXPECTED_ARCHITECTURES:
        errors.append("manifest target architectures must contain exactly arm64 and x86_64")
    if not isinstance(validation, dict):
        errors.append("manifest validation must be an object")
    else:
        if validation.get("wslStaticValidation") != "passed":
            errors.append("manifest must record wslStaticValidation as passed")
        if validation.get("macosNativeValidation") != "pending":
            errors.append("manifest must record macosNativeValidation as pending")
    return errors


def command_version(arguments: Sequence[str]) -> str:
    result = run(arguments, check=False)
    return result.stdout.strip().splitlines()[0] if result.stdout.strip() else "unknown"


def read_os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    path = Path("/etc/os-release")
    if not path.is_file():
        return values
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key] = value.strip().strip('"')
    return values


def read_apt_versions(path: Path) -> list[dict[str, str]]:
    if not path.is_file():
        return []
    packages: list[dict[str, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        name, separator, version = line.partition("=")
        if separator and name and version:
            packages.append({"name": name, "version": version})
    return packages


def build_manifest(
    metadata: ProjectMetadata,
    binary: Path,
    dmg: Path,
    apt_versions: Path,
    arguments: argparse.Namespace,
) -> dict[str, Any]:
    os_release = read_os_release()
    git_result = run(["git", "-C", str(metadata.root), "rev-parse", "HEAD"], check=False)
    git_commit = git_result.stdout.strip() if git_result.returncode == 0 else None
    return {
        "schemaVersion": 1,
        "builtAtUtc": arguments.built_at_utc,
        "project": {
            "productName": metadata.product_name,
            "version": metadata.version,
            "bundleIdentifier": metadata.identifier,
            "binaryName": metadata.binary_name,
            "deepLinkSchemes": list(metadata.deep_link_schemes),
            "gitCommit": git_commit,
            "worktreePolicy": "unchecked",
        },
        "target": {
            "name": "universal-apple-darwin",
            "architectures": ["arm64", "x86_64"],
            "minimumMacOSVersion": metadata.minimum_system_version,
        },
        "host": {
            "wslVersion": 2,
            "platform": platform.platform(),
            "kernel": platform.release(),
            "architecture": platform.machine(),
            "distribution": os_release.get("PRETTY_NAME", "unknown"),
            "aptPackages": read_apt_versions(apt_versions),
        },
        "runtimes": {
            "mise": command_version([arguments.mise_bin, "--version"]),
            "node": command_version(["node", "--version"]),
            "pnpm": command_version(["pnpm", "--version"]),
            "python": command_version(["python", "--version"]),
            "rustc": command_version(["rustc", "--version"]),
            "cargo": command_version(["cargo", "--version"]),
            "tauriCli": metadata.tauri_cli_lock_version,
        },
        "toolchain": {
            "sdk": {
                "version": arguments.sdk_version,
                "url": arguments.sdk_url,
                "sha256": arguments.sdk_sha256,
            },
            "osxcross": {
                "commit": arguments.osxcross_commit,
                "buildFlavor": arguments.osxcross_build_flavor,
            },
            "libdmgHfsplus": {
                "commit": arguments.libdmg_commit,
                "fileVault": "disabled",
                "license": "GPL-3.0",
                "upstreamStatus": "highly-experimental",
            },
            "rcodesign": {
                "version": arguments.rcodesign_version,
                "archiveSha256": arguments.rcodesign_sha256,
            },
        },
        "signing": {
            "app": "ad-hoc",
            "dmg": "ad-hoc",
            "notarized": False,
        },
        "artifact": {
            "fileName": dmg.name,
            "sizeBytes": dmg.stat().st_size,
            "sha256": sha256_file(dmg),
            "format": "UDIF",
            "trailer": "koly",
            "binarySha256": sha256_file(binary),
        },
        "validation": {
            "wslStaticValidation": "passed",
            "macosNativeValidation": "pending",
        },
        "warnings": [
            "The macOS SDK came from a fixed third-party release.",
            "The DMG was produced with experimental non-Apple tooling.",
            "The app and DMG use ad-hoc signatures and are not notarized.",
            "Mount, installation, launch, codesign, and Gatekeeper checks on a real Mac are pending.",
        ],
    }


def report(errors: Sequence[str]) -> int:
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print("static artifact verification passed")
    return 0


def add_manifest_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--binary", required=True)
    parser.add_argument("--dmg", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--apt-versions", required=True)
    parser.add_argument("--built-at-utc", required=True)
    parser.add_argument("--mise-bin", required=True)
    parser.add_argument("--sdk-version", required=True)
    parser.add_argument("--sdk-url", required=True)
    parser.add_argument("--sdk-sha256", required=True)
    parser.add_argument("--osxcross-commit", required=True)
    parser.add_argument("--osxcross-build-flavor", required=True)
    parser.add_argument("--libdmg-commit", required=True)
    parser.add_argument("--rcodesign-version", required=True)
    parser.add_argument("--rcodesign-sha256", required=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify WSL-built macOS artifacts.")
    parser.add_argument("--project-root", default=".")
    subparsers = parser.add_subparsers(dest="command", required=True)

    binary_parser = subparsers.add_parser("binary")
    binary_parser.add_argument("path")
    binary_parser.add_argument("--deployment-target", required=True)

    app_parser = subparsers.add_parser("app")
    app_parser.add_argument("path")
    app_parser.add_argument("--deployment-target", required=True)

    dmg_parser = subparsers.add_parser("dmg")
    dmg_parser.add_argument("path")

    manifest_parser = subparsers.add_parser("manifest")
    add_manifest_arguments(manifest_parser)

    manifest_check_parser = subparsers.add_parser("manifest-check")
    manifest_check_parser.add_argument("--manifest", required=True)
    manifest_check_parser.add_argument("--dmg", required=True)
    args = parser.parse_args()

    try:
        metadata = require_project(Path(args.project_root))
        if args.command == "binary":
            return report(
                verify_universal_binary(Path(args.path).resolve(), args.deployment_target)
            )
        if args.command == "app":
            return report(
                verify_app(metadata, Path(args.path).resolve(), args.deployment_target)
            )
        if args.command == "dmg":
            return report(verify_dmg(Path(args.path).resolve()))
        if args.command == "manifest":
            manifest = build_manifest(
                metadata,
                Path(args.binary).resolve(),
                Path(args.dmg).resolve(),
                Path(args.apt_versions).resolve(),
                args,
            )
            output = Path(args.output).resolve()
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
            print(output)
            return 0
        if args.command == "manifest-check":
            return report(
                verify_manifest(Path(args.manifest).resolve(), Path(args.dmg).resolve())
            )
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    raise AssertionError(f"unhandled command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
