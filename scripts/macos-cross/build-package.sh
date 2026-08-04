#!/usr/bin/env bash
set -euo pipefail
# shellcheck source-path=SCRIPTDIR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"
# shellcheck source=macos-cross-env.sh
source "$SCRIPT_DIR/macos-cross-env.sh"

cleanup_paths=()
publish_dir=""
cleanup() {
  local path
  for path in "${cleanup_paths[@]}"; do
    if [[ -e "$path" ]]; then
      fyagent_remove_owned_tree "$path" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"
    fi
  done
  if [[ -n "$publish_dir" && -e "$publish_dir" ]]; then
    fyagent_remove_owned_tree "$publish_dir" "$PROJECT_ROOT/dist-macos"
  fi
}
trap cleanup EXIT

assert_mise_tool() {
  local command_name="$1"
  local expected
  local resolved
  resolved="$(command -v "$command_name")"
  expected="$($FYAGENT_MISE_BIN --cd "$PROJECT_ROOT" which "$command_name")"
  case "$resolved" in
    /mnt/*) fyagent_die "$command_name resolved to a Windows-mounted path: $resolved" ;;
  esac
  [[ "$(realpath -e -- "$resolved")" == "$(realpath -e -- "$expected")" ]] || \
    fyagent_die "$command_name does not match global mise project resolution: $resolved"
}

assert_mise_tool node
assert_mise_tool pnpm
assert_mise_tool python
assert_mise_tool rustc
assert_mise_tool cargo

cd "$PROJECT_ROOT"
python "$SCRIPT_DIR/preflight.py" "$PROJECT_ROOT"

fyagent_log "installing JavaScript dependencies from pnpm-lock.yaml"
CI=true pnpm install --frozen-lockfile

fyagent_log "building the Tauri Universal Mach-O (arm64 + x86_64)"
pnpm tauri build --target universal-apple-darwin --no-bundle --ci

product_name="$(python "$SCRIPT_DIR/preflight.py" "$PROJECT_ROOT" --field product_name)"
version="$(python "$SCRIPT_DIR/preflight.py" "$PROJECT_ROOT" --field version)"
binary_name="$(python "$SCRIPT_DIR/preflight.py" "$PROJECT_ROOT" --field binary_name)"
binary_path="$PROJECT_ROOT/src-tauri/target/universal-apple-darwin/release/$binary_name"

python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" \
  binary "$binary_path" --deployment-target "$FYAGENT_MACOSX_DEPLOYMENT_TARGET"

package_root="$(mktemp -d "${FYAGENT_MACOS_CROSS_CACHE_ROOT}/.package.XXXXXX")"
cleanup_paths+=("$package_root")
app_output_dir="$package_root/app"
app_path="$(python "$SCRIPT_DIR/make_app.py" "$binary_path" \
  --project-root "$PROJECT_ROOT" --output-dir "$app_output_dir")"

fyagent_log "creating and verifying the ad-hoc app signature"
rcodesign sign "$app_path"
python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" \
  app "$app_path" --deployment-target "$FYAGENT_MACOSX_DEPLOYMENT_TARGET"

artifact_base="${product_name}-${version}-macOS-universal-adhoc-unnotarized-experimental.dmg"
temporary_dmg="$package_root/$artifact_base"
"$SCRIPT_DIR/make-dmg.sh" --app "$app_path" --output "$temporary_dmg" --volume-name "$product_name"

fyagent_log "creating and verifying the ad-hoc DMG signature"
rcodesign sign "$temporary_dmg"
python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" dmg "$temporary_dmg"

temporary_checksum="$package_root/${artifact_base}.sha256"
temporary_manifest="$package_root/${artifact_base}.manifest.json"
(
  cd "$package_root"
  sha256sum "$artifact_base" >"$(basename "$temporary_checksum")"
  sha256sum -c "$(basename "$temporary_checksum")"
)

python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" manifest \
  --binary "$binary_path" \
  --dmg "$temporary_dmg" \
  --output "$temporary_manifest" \
  --apt-versions "${FYAGENT_MACOS_CROSS_STATE_ROOT}/host/apt-packages.txt" \
  --built-at-utc "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --mise-bin "$FYAGENT_MISE_BIN" \
  --sdk-version "$FYAGENT_MACOS_SDK_VERSION" \
  --sdk-url "$FYAGENT_MACOS_SDK_URL" \
  --sdk-sha256 "$FYAGENT_MACOS_SDK_SHA256" \
  --osxcross-commit "$FYAGENT_OSXCROSS_COMMIT" \
  --osxcross-build-flavor "$FYAGENT_OSXCROSS_BUILD_FLAVOR" \
  --libdmg-commit "$FYAGENT_LIBDMG_COMMIT" \
  --rcodesign-version "$FYAGENT_RCODESIGN_VERSION" \
  --rcodesign-sha256 "$FYAGENT_RCODESIGN_SHA256"

python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" manifest-check \
  --manifest "$temporary_manifest" --dmg "$temporary_dmg"

dist_dir="$PROJECT_ROOT/dist-macos"
mkdir -p "$dist_dir"
final_dmg="$dist_dir/$artifact_base"
final_checksum="$dist_dir/${artifact_base}.sha256"
final_manifest="$dist_dir/${artifact_base}.manifest.json"

# All fallible build/sign/verification work is complete before any final path is replaced.
publish_dir="$(mktemp -d "$dist_dir/.publish.XXXXXX")"
cp -p -- "$temporary_dmg" "$publish_dir/$artifact_base"
cp -p -- "$temporary_checksum" "$publish_dir/${artifact_base}.sha256"
cp -p -- "$temporary_manifest" "$publish_dir/${artifact_base}.manifest.json"
(
  cd "$publish_dir"
  sha256sum -c "${artifact_base}.sha256"
)
mv -f -- "$publish_dir/${artifact_base}.sha256" "$final_checksum"
mv -f -- "$publish_dir/${artifact_base}.manifest.json" "$final_manifest"
mv -f -- "$publish_dir/$artifact_base" "$final_dmg"
rmdir "$publish_dir"
publish_dir=""

(
  cd "$dist_dir"
  sha256sum -c "$(basename "$final_checksum")"
)
python "$SCRIPT_DIR/verify_artifacts.py" --project-root "$PROJECT_ROOT" manifest-check \
  --manifest "$final_manifest" --dmg "$final_dmg"

cat <<EOF

FyAgent WSL macOS Universal DMG build completed.
  DMG:      $final_dmg
  SHA256:   $final_checksum
  Manifest: $final_manifest

This artifact is ad-hoc signed, unnotarized, and experimental. Native macOS
mount, installation, launch, codesign, and Gatekeeper acceptance are pending.
EOF
