#!/usr/bin/env python3
from __future__ import annotations

import json
import plistlib
import re
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


EXPECTED_PRODUCT_NAME = "FyAgent"
EXPECTED_BINARY_NAME = "fyagent"
EXPECTED_IDENTIFIER = "com.fyagent.desktop"
EXPECTED_DEEP_LINK_SCHEMES = ("fyagent",)
EXPECTED_MINIMUM_SYSTEM_VERSION = "12.0"
EXPECTED_NODE_VERSION = "22.12.0"
EXPECTED_PNPM_VERSION = "10.12.3"
EXPECTED_PYTHON_VERSION = "3.12.8"
EXPECTED_RUST_VERSION = "1.95.0"
EXPECTED_TAURI_CLI_VERSION = "2.8.1"


@dataclass(frozen=True)
class ProjectMetadata:
    root: Path
    product_name: str
    version: str
    identifier: str
    binary_name: str
    minimum_system_version: str
    icon_path: Path
    deep_link_schemes: tuple[str, ...]
    node_version: str
    package_manager: str
    rust_channel: str
    tauri_cli_lock_version: str
    tauri_api_lock_version: str
    tauri_core_lock_version: str
    tauri_build_lock_version: str
    wry_lock_version: str
    tao_lock_version: str

    def to_json(self) -> dict[str, Any]:
        value = asdict(self)
        value["root"] = str(self.root)
        value["icon_path"] = str(self.icon_path)
        value["deep_link_schemes"] = list(self.deep_link_schemes)
        return value


@dataclass(frozen=True)
class Inspection:
    metadata: ProjectMetadata | None
    errors: tuple[str, ...]
    warnings: tuple[str, ...]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"expected an object in {path}")
    return value


def read_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as stream:
        value = tomllib.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"expected a table in {path}")
    return value


def _nonempty(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, (list, dict, str)):
        return bool(value)
    return True


def _pnpm_importer_version(lock_path: Path, package_name: str) -> str:
    text = lock_path.read_text(encoding="utf-8")
    escaped = re.escape(package_name)
    pattern = rf"(?ms)^\s{{6}}['\"]?{escaped}['\"]?:\s*$.*?^\s{{8}}version:\s*([^\s]+)\s*$"
    match = re.search(pattern, text)
    if not match:
        return ""
    value = match.group(1).strip("'\"")
    return value.split("(", 1)[0]


def _cargo_lock_version(lock_path: Path, package_name: str) -> str:
    lock = read_toml(lock_path)
    packages = lock.get("package", [])
    if not isinstance(packages, list):
        return ""
    versions = [
        str(package.get("version", ""))
        for package in packages
        if isinstance(package, dict) and package.get("name") == package_name
    ]
    return versions[-1] if versions else ""


def _normalize_rust_version(value: str) -> str:
    if re.fullmatch(r"\d+\.\d+", value):
        return f"{value}.0"
    return value


def inspect_project(root: Path) -> Inspection:
    root = root.expanduser().resolve()
    errors: list[str] = []
    warnings: list[str] = []
    required_paths = {
        "package.json": root / "package.json",
        ".node-version": root / ".node-version",
        "rust-toolchain.toml": root / "rust-toolchain.toml",
        "mise.toml": root / "mise.toml",
        "pnpm-lock.yaml": root / "pnpm-lock.yaml",
        "src-tauri/tauri.conf.json": root / "src-tauri" / "tauri.conf.json",
        "src-tauri/Cargo.toml": root / "src-tauri" / "Cargo.toml",
        "src-tauri/Cargo.lock": root / "src-tauri" / "Cargo.lock",
    }
    for label, path in required_paths.items():
        if not path.is_file():
            errors.append(f"missing required file: {label}")
    if errors:
        return Inspection(None, tuple(errors), tuple(warnings))

    try:
        package = read_json(required_paths["package.json"])
        rust_toolchain = read_toml(required_paths["rust-toolchain.toml"])
        mise = read_toml(required_paths["mise.toml"])
        tauri = read_json(required_paths["src-tauri/tauri.conf.json"])
        cargo = read_toml(required_paths["src-tauri/Cargo.toml"])
    except (OSError, ValueError, tomllib.TOMLDecodeError, json.JSONDecodeError) as error:
        return Inspection(None, (str(error),), tuple(warnings))

    bundle = tauri.get("bundle", {})
    if not isinstance(bundle, dict):
        errors.append("tauri.conf.json: bundle must be an object")
        bundle = {}
    macos = bundle.get("macOS", {})
    if not isinstance(macos, dict):
        errors.append("tauri.conf.json: bundle.macOS must be an object")
        macos = {}

    cargo_package = cargo.get("package", {})
    if not isinstance(cargo_package, dict):
        errors.append("src-tauri/Cargo.toml: package must be a table")
        cargo_package = {}

    product_name = str(tauri.get("productName", ""))
    version = str(tauri.get("version", ""))
    identifier = str(tauri.get("identifier", ""))
    binary_name = str(cargo_package.get("name", ""))
    minimum_system_version = str(macos.get("minimumSystemVersion", ""))
    node_version = required_paths[".node-version"].read_text(encoding="utf-8").strip()
    package_manager = str(package.get("packageManager", ""))
    rust_channel = str(rust_toolchain.get("toolchain", {}).get("channel", ""))

    identity_checks = {
        "productName": (product_name, EXPECTED_PRODUCT_NAME),
        "Cargo package.name": (binary_name, EXPECTED_BINARY_NAME),
        "identifier": (identifier, EXPECTED_IDENTIFIER),
        "bundle.macOS.minimumSystemVersion": (
            minimum_system_version,
            EXPECTED_MINIMUM_SYSTEM_VERSION,
        ),
        ".node-version": (node_version, EXPECTED_NODE_VERSION),
        "packageManager": (package_manager, f"pnpm@{EXPECTED_PNPM_VERSION}"),
    }
    for label, (actual, expected) in identity_checks.items():
        if actual != expected:
            errors.append(f"{label} must be {expected!r}; found {actual!r}")

    if _normalize_rust_version(rust_channel) != EXPECTED_RUST_VERSION:
        errors.append(
            f"rust-toolchain channel must resolve to {EXPECTED_RUST_VERSION}; found {rust_channel!r}"
        )

    versions = {
        "tauri.conf.json": version,
        "package.json": str(package.get("version", "")),
        "src-tauri/Cargo.toml": str(cargo_package.get("version", "")),
    }
    if len(set(versions.values())) != 1 or not version:
        errors.append(
            "project version mismatch: "
            + ", ".join(f"{source}={value!r}" for source, value in versions.items())
        )
    cargo_lock_app_version = _cargo_lock_version(
        required_paths["src-tauri/Cargo.lock"], binary_name
    )
    if cargo_lock_app_version != version:
        errors.append(
            "project version mismatch: "
            f"src-tauri/Cargo.lock={cargo_lock_app_version!r}, expected {version!r}"
        )

    tools = mise.get("tools", {})
    if not isinstance(tools, dict):
        errors.append("mise.toml: tools must be a table")
        tools = {}
    expected_mise_tools = {
        "node": EXPECTED_NODE_VERSION,
        "pnpm": EXPECTED_PNPM_VERSION,
        "python": EXPECTED_PYTHON_VERSION,
    }
    for tool_name, expected in expected_mise_tools.items():
        if tools.get(tool_name) != expected:
            errors.append(f"mise.toml tools.{tool_name} must be {expected!r}")
    rust_mise = tools.get("rust")
    if not isinstance(rust_mise, dict):
        errors.append("mise.toml tools.rust must be a structured tool definition")
    else:
        if str(rust_mise.get("version", "")) != EXPECTED_RUST_VERSION:
            errors.append(f"mise.toml Rust version must be {EXPECTED_RUST_VERSION}")
        expected_macos_targets = {"aarch64-apple-darwin", "x86_64-apple-darwin"}
        actual_targets = set(rust_mise.get("targets", []))
        if not expected_macos_targets.issubset(actual_targets):
            errors.append("mise.toml Rust targets must include both macOS architectures")

    icon_entries = bundle.get("icon", [])
    if not isinstance(icon_entries, list):
        errors.append("bundle.icon must be an array")
        icon_entries = []
    icon_relative = next((str(value) for value in icon_entries if str(value).endswith(".icns")), "")
    icon_path = root / "src-tauri" / icon_relative
    if not icon_relative or not icon_path.is_file():
        errors.append("bundle.icon must reference an existing .icns file")

    plugins = tauri.get("plugins", {})
    deep_link = plugins.get("deep-link", {}) if isinstance(plugins, dict) else {}
    desktop = deep_link.get("desktop", {}) if isinstance(deep_link, dict) else {}
    schemes = desktop.get("schemes", []) if isinstance(desktop, dict) else []
    if not isinstance(schemes, list):
        schemes = []
    deep_link_schemes = tuple(str(value) for value in schemes)
    if deep_link_schemes != EXPECTED_DEEP_LINK_SCHEMES:
        errors.append(
            f"deep-link schemes must be exactly {list(EXPECTED_DEEP_LINK_SCHEMES)!r}; "
            f"found {list(deep_link_schemes)!r}"
        )

    unsupported = {
        "bundle.externalBin": bundle.get("externalBin"),
        "bundle.resources": bundle.get("resources"),
        "bundle.fileAssociations": bundle.get("fileAssociations"),
        "bundle.macOS.frameworks": macos.get("frameworks"),
        "bundle.macOS.files": macos.get("files"),
        "bundle.macOS.entitlements": macos.get("entitlements"),
        "bundle.macOS.signingIdentity": macos.get("signingIdentity"),
        "bundle.macOS.dmg": macos.get("dmg"),
    }
    for config_path, value in unsupported.items():
        if _nonempty(value):
            errors.append(
                f"{config_path} is non-empty; the manual WSL app/DMG assembler does not support it"
            )

    cargo_bins = cargo.get("bin", [])
    if _nonempty(cargo_bins):
        errors.append("explicit [[bin]] targets are unsupported; the workflow expects the package binary only")

    custom_info_plist = root / "src-tauri" / "Info.plist"
    if custom_info_plist.is_file():
        try:
            with custom_info_plist.open("rb") as stream:
                custom_info = plistlib.load(stream)
            if not isinstance(custom_info, dict):
                errors.append("src-tauri/Info.plist must contain a dictionary")
        except (OSError, plistlib.InvalidFileException) as error:
            errors.append(f"cannot parse src-tauri/Info.plist: {error}")

    tauri_cli_lock_version = _pnpm_importer_version(
        required_paths["pnpm-lock.yaml"], "@tauri-apps/cli"
    )
    if tauri_cli_lock_version != EXPECTED_TAURI_CLI_VERSION:
        errors.append(
            f"pnpm lock must resolve @tauri-apps/cli {EXPECTED_TAURI_CLI_VERSION}; "
            f"found {tauri_cli_lock_version!r}"
        )

    metadata = ProjectMetadata(
        root=root,
        product_name=product_name,
        version=version,
        identifier=identifier,
        binary_name=binary_name,
        minimum_system_version=minimum_system_version,
        icon_path=icon_path,
        deep_link_schemes=deep_link_schemes,
        node_version=node_version,
        package_manager=package_manager,
        rust_channel=rust_channel,
        tauri_cli_lock_version=tauri_cli_lock_version,
        tauri_api_lock_version=_pnpm_importer_version(
            required_paths["pnpm-lock.yaml"], "@tauri-apps/api"
        ),
        tauri_core_lock_version=_cargo_lock_version(
            required_paths["src-tauri/Cargo.lock"], "tauri"
        ),
        tauri_build_lock_version=_cargo_lock_version(
            required_paths["src-tauri/Cargo.lock"], "tauri-build"
        ),
        wry_lock_version=_cargo_lock_version(required_paths["src-tauri/Cargo.lock"], "wry"),
        tao_lock_version=_cargo_lock_version(required_paths["src-tauri/Cargo.lock"], "tao"),
    )
    return Inspection(metadata, tuple(errors), tuple(warnings))


def require_project(root: Path) -> ProjectMetadata:
    inspection = inspect_project(root)
    if inspection.errors:
        raise ValueError("; ".join(inspection.errors))
    if inspection.metadata is None:
        raise ValueError("project metadata is unavailable")
    return inspection.metadata
