from __future__ import annotations

import json
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


SCRIPT_DIR = Path(__file__).resolve().parents[1]
PROJECT_ROOT = SCRIPT_DIR.parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

import make_app  # noqa: E402
import project_metadata  # noqa: E402
import verify_artifacts  # noqa: E402


class ShellCompositionTests(unittest.TestCase):
    def test_constants_can_be_sourced_more_than_once(self) -> None:
        constants = SCRIPT_DIR / "constants.sh"
        result = subprocess.run(
            ["bash", "-c", 'source "$1"; source "$1"', "bash", str(constants)],
            capture_output=True,
            check=False,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_public_entrypoint_help_and_rejects_target_options(self) -> None:
        entrypoint = SCRIPT_DIR / "build-universal-dmg.sh"
        help_result = subprocess.run(
            [str(entrypoint), "--help"],
            capture_output=True,
            check=False,
            text=True,
        )
        target_result = subprocess.run(
            [str(entrypoint), "--target", "aarch64-apple-darwin"],
            capture_output=True,
            check=False,
            text=True,
        )
        self.assertEqual(help_result.returncode, 0, help_result.stderr)
        self.assertIn("Only universal-apple-darwin is supported", help_result.stdout)
        self.assertNotEqual(target_result.returncode, 0)
        self.assertIn("unknown argument: --target", target_result.stderr)

    def test_public_entrypoint_requires_global_mise(self) -> None:
        entrypoint = SCRIPT_DIR / "build-universal-dmg.sh"
        environment = os.environ.copy()
        environment["PATH"] = "/usr/bin:/bin"
        result = subprocess.run(
            [str(entrypoint)],
            capture_output=True,
            check=False,
            env=environment,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("global mise is required; install mise and rerun", result.stderr)

    def test_checksum_mismatch_discards_partial_download(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            fake_bin = temporary_root / "bin"
            fake_bin.mkdir()
            fake_curl = fake_bin / "curl"
            fake_curl.write_text(
                """#!/usr/bin/env bash
set -euo pipefail
output=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf 'unexpected payload' >"$output"
""",
                encoding="utf-8",
            )
            fake_curl.chmod(0o755)
            destination = temporary_root / "download.bin"
            environment = os.environ.copy()
            environment["PATH"] = f"{fake_bin}:{environment['PATH']}"
            result = subprocess.run(
                [
                    "bash",
                    "-c",
                    'source "$1"; fyagent_download_verified '
                    "https://example.invalid/input "
                    f"{'0' * 64} "
                    '"$2"',
                    "bash",
                    str(SCRIPT_DIR / "lib.sh"),
                    str(destination),
                ],
                capture_output=True,
                check=False,
                env=environment,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("checksum mismatch", result.stderr)
            self.assertFalse(destination.exists())
            self.assertFalse(Path(f"{destination}.part").exists())


class ProjectFixture:
    def __init__(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.root = Path(self._temporary.name)
        for relative in (
            ".node-version",
            "mise.toml",
            "package.json",
            "pnpm-lock.yaml",
            "rust-toolchain.toml",
            "src-tauri/Cargo.toml",
            "src-tauri/Cargo.lock",
            "src-tauri/tauri.conf.json",
            "src-tauri/icons/icon.icns",
        ):
            source = PROJECT_ROOT / relative
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)

    def close(self) -> None:
        self._temporary.cleanup()


class ProjectMetadataTests(unittest.TestCase):
    def test_current_project_passes_preflight(self) -> None:
        inspection = project_metadata.inspect_project(PROJECT_ROOT)
        self.assertEqual(inspection.errors, ())
        self.assertIsNotNone(inspection.metadata)
        assert inspection.metadata is not None
        self.assertEqual(inspection.metadata.product_name, "FyAgent")
        self.assertEqual(inspection.metadata.deep_link_schemes, ("fyagent",))
        self.assertEqual(inspection.metadata.tauri_cli_lock_version, "2.8.1")

    def test_manual_bundle_rejects_every_unhandled_tauri_field(self) -> None:
        cases = {
            "externalBin": ["bin/helper"],
            "resources": ["resources/example.txt"],
            "fileAssociations": [{"ext": ["fyagent"]}],
            "macOS.frameworks": ["Example.framework"],
            "macOS.files": {"Extra": "extra.txt"},
            "macOS.entitlements": "entitlements.plist",
            "macOS.signingIdentity": "Developer ID Application: Example",
            "macOS.dmg": {"background": "background.png"},
        }
        for field, value in cases.items():
            with self.subTest(field=field):
                fixture = ProjectFixture()
                self.addCleanup(fixture.close)
                config_path = fixture.root / "src-tauri" / "tauri.conf.json"
                config = json.loads(config_path.read_text(encoding="utf-8"))
                if field.startswith("macOS."):
                    config["bundle"]["macOS"][field.split(".", 1)[1]] = value
                else:
                    config["bundle"][field] = value
                config_path.write_text(json.dumps(config), encoding="utf-8")

                inspection = project_metadata.inspect_project(fixture.root)
                self.assertTrue(
                    any(
                        f"bundle.{field} is non-empty" in error
                        for error in inspection.errors
                    ),
                    inspection.errors,
                )

    def test_manual_bundle_rejects_an_explicit_extra_binary(self) -> None:
        fixture = ProjectFixture()
        self.addCleanup(fixture.close)
        cargo_path = fixture.root / "src-tauri" / "Cargo.toml"
        with cargo_path.open("a", encoding="utf-8") as stream:
            stream.write('\n[[bin]]\nname = "extra"\npath = "src/extra.rs"\n')

        inspection = project_metadata.inspect_project(fixture.root)
        self.assertTrue(
            any("explicit [[bin]] targets are unsupported" in error for error in inspection.errors)
        )

    def test_project_version_must_match_cargo_lock(self) -> None:
        fixture = ProjectFixture()
        self.addCleanup(fixture.close)
        lock_path = fixture.root / "src-tauri" / "Cargo.lock"
        lock_text = lock_path.read_text(encoding="utf-8")
        old = 'name = "fyagent"\nversion = "0.2.0"'
        self.assertIn(old, lock_text)
        lock_path.write_text(
            lock_text.replace(old, 'name = "fyagent"\nversion = "9.9.9"', 1),
            encoding="utf-8",
        )

        inspection = project_metadata.inspect_project(fixture.root)
        self.assertTrue(
            any("src-tauri/Cargo.lock='9.9.9'" in error for error in inspection.errors)
        )

    def test_custom_info_plist_overrides_tauri_defaults(self) -> None:
        fixture = ProjectFixture()
        self.addCleanup(fixture.close)
        custom_path = fixture.root / "src-tauri" / "Info.plist"
        with custom_path.open("wb") as stream:
            plistlib.dump({"NSCameraUsageDescription": "Test camera"}, stream)

        info = make_app.build_info_plist(fixture.root)
        self.assertEqual(info["CFBundleIdentifier"], "com.fyagent.desktop")
        self.assertEqual(info["NSCameraUsageDescription"], "Test camera")
        self.assertEqual(
            info["CFBundleURLTypes"][0]["CFBundleURLSchemes"], ["fyagent"]
        )


class ArtifactVerificationTests(unittest.TestCase):
    def test_udif_magic_classifies_a_koly_trailer(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            dmg = Path(temporary) / "fixture.dmg"
            dmg.write_bytes(b"payload" + b"koly" + bytes(508))
            result = subprocess.run(
                [
                    "file",
                    "--brief",
                    "--magic-file",
                    str(verify_artifacts.UDIF_MAGIC_FILE),
                    str(dmg),
                ],
                capture_output=True,
                check=False,
                text=True,
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Apple Disk Image", result.stdout)

    def test_binary_rejects_missing_universal_slice_and_linux_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "fyagent"
            binary.write_bytes(b"fixture")
            os.chmod(binary, 0o755)
            outputs = [
                subprocess.CompletedProcess([], 0, "Mach-O universal binary\n"),
                subprocess.CompletedProcess([], 0, "arm64\n"),
                subprocess.CompletedProcess([], 0, "/home/user/libbad.so\n"),
                subprocess.CompletedProcess([], 0, "minos 12.0\n"),
                subprocess.CompletedProcess([], 0, "no deployment command\n"),
            ]
            with mock.patch.object(verify_artifacts, "run", side_effect=outputs):
                errors = verify_artifacts.verify_universal_binary(binary, "12.0")
        self.assertTrue(any("exactly arm64 and x86_64" in error for error in errors))
        self.assertTrue(any("contamination" in error for error in errors))
        self.assertTrue(any("both Universal slices" in error for error in errors))

    def test_binary_checks_deployment_target_per_slice(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "fyagent"
            binary.write_bytes(b"fixture")
            os.chmod(binary, 0o755)
            outputs = [
                subprocess.CompletedProcess([], 0, "Mach-O universal binary\n"),
                subprocess.CompletedProcess([], 0, "arm64 x86_64\n"),
                subprocess.CompletedProcess([], 0, "/usr/lib/libSystem.B.dylib\n"),
                subprocess.CompletedProcess([], 0, "minos 12.0\n"),
                subprocess.CompletedProcess([], 0, "minos 12.0\n"),
            ]
            with mock.patch.object(verify_artifacts, "run", side_effect=outputs):
                errors = verify_artifacts.verify_universal_binary(binary, "12.0")
        self.assertEqual(errors, [])

    def test_dmg_requires_koly_trailer(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            dmg = Path(temporary) / "FyAgent.dmg"
            dmg.write_bytes(b"not-a-udif" + bytes(512))
            outputs = [
                subprocess.CompletedProcess([], 0, "Apple Disk Image (UDIF)\n"),
                subprocess.CompletedProcess(
                    [], 0, "CodeSignatureFlags(ADHOC)\n  cms: null\n"
                ),
            ]
            with mock.patch.object(verify_artifacts, "run", side_effect=outputs):
                errors = verify_artifacts.verify_dmg(dmg)
        self.assertIn("UDIF koly trailer is missing", errors)

    def test_valid_dmg_passes_static_checks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            dmg = Path(temporary) / "FyAgent.dmg"
            dmg.write_bytes(b"payload" + b"koly" + bytes(508))
            outputs = [
                subprocess.CompletedProcess([], 0, "Apple Disk Image (UDIF)\n"),
                subprocess.CompletedProcess(
                    [], 0, "CodeSignatureFlags(ADHOC)\n  cms: null\n"
                ),
            ]
            with mock.patch.object(verify_artifacts, "run", side_effect=outputs):
                errors = verify_artifacts.verify_dmg(dmg)
        self.assertEqual(errors, [])

    def test_dmg_rejects_a_non_adhoc_signature(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            dmg = Path(temporary) / "FyAgent.dmg"
            dmg.write_bytes(b"payload" + b"koly" + bytes(508))
            outputs = [
                subprocess.CompletedProcess([], 0, "Apple Disk Image (UDIF)\n"),
                subprocess.CompletedProcess([], 0, "cms: signed identity\n"),
            ]
            with mock.patch.object(verify_artifacts, "run", side_effect=outputs):
                errors = verify_artifacts.verify_dmg(dmg)
        self.assertIn("DMG signature is not marked ad-hoc", errors)

    def test_manifest_generation_and_mismatch_detection(self) -> None:
        metadata = project_metadata.require_project(PROJECT_ROOT)
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            binary = temporary_root / "fyagent"
            dmg = temporary_root / "FyAgent.dmg"
            apt_versions = temporary_root / "apt.txt"
            manifest_path = temporary_root / "FyAgent.dmg.manifest.json"
            binary.write_bytes(b"universal-binary")
            dmg.write_bytes(b"payload" + b"koly" + bytes(508))
            apt_versions.write_text("clang=1.2.3\n", encoding="utf-8")
            arguments = SimpleNamespace(
                built_at_utc="2026-08-03T00:00:00Z",
                mise_bin="mise",
                sdk_version="14.5",
                sdk_url="https://example.invalid/MacOSX14.5.sdk.tar.xz",
                sdk_sha256="a" * 64,
                osxcross_commit="b" * 40,
                osxcross_build_flavor="llvm",
                libdmg_commit="c" * 40,
                rcodesign_version="0.29.0",
                rcodesign_sha256="d" * 64,
            )
            with mock.patch.object(
                verify_artifacts,
                "run",
                return_value=subprocess.CompletedProcess([], 0, "test-version\n"),
            ):
                manifest = verify_artifacts.build_manifest(
                    metadata, binary, dmg, apt_versions, arguments
                )
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            self.assertEqual(
                verify_artifacts.verify_manifest(manifest_path, dmg), []
            )
            self.assertEqual(manifest["host"]["wslVersion"], 2)
            self.assertEqual(
                manifest["toolchain"]["libdmgHfsplus"]["fileVault"], "disabled"
            )
            dmg.write_bytes(dmg.read_bytes() + b"tampered")
            errors = verify_artifacts.verify_manifest(manifest_path, dmg)
        self.assertTrue(
            any("manifest artifact.sizeBytes" in error for error in errors)
        )
        self.assertTrue(any("manifest artifact.sha256" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
