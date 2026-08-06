#!/usr/bin/env bash
set -euo pipefail
# shellcheck source-path=SCRIPTDIR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  cat <<'USAGE'
Usage: make-dmg.sh --app PATH --output PATH --volume-name NAME

Internal helper. Creates an unsigned experimental UDIF image; the caller must
sign and re-verify it before publication.
USAGE
}

app_path=""
output_path=""
volume_name=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --app) app_path="${2:?missing value for --app}"; shift 2 ;;
    --output) output_path="${2:?missing value for --output}"; shift 2 ;;
    --volume-name) volume_name="${2:?missing value for --volume-name}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fyagent_die "unknown argument: $1"; usage >&2; exit 2 ;;
  esac
done

[[ -d "$app_path" ]] || fyagent_die "app bundle not found: $app_path"
[[ -n "$output_path" ]] || fyagent_die "--output is required"
[[ -n "$volume_name" ]] || fyagent_die "--volume-name is required"
[[ -x "${FYAGENT_LIBDMG_INSTALL_DIR}/bin/dmg" ]] || fyagent_die "libdmg is not provisioned"
fyagent_require_command xorriso

work_dir="$(mktemp -d "${FYAGENT_MACOS_CROSS_CACHE_ROOT}/.dmg.XXXXXX")"
cleanup() {
  fyagent_remove_owned_tree "$work_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"
}
trap cleanup EXIT

stage_dir="$work_dir/stage"
raw_image="$work_dir/${volume_name}.iso"
mkdir -p "$stage_dir" "$(dirname "$output_path")"
cp -a -- "$app_path" "$stage_dir/"
ln -s /Applications "$stage_dir/Applications"

xorriso -as mkisofs \
  -iso-level 3 -R -J -hfsplus \
  -V "$volume_name" \
  -o "$raw_image" \
  "$stage_dir"

"${FYAGENT_LIBDMG_INSTALL_DIR}/bin/dmg" dmg "$raw_image" "$output_path"
[[ -s "$output_path" ]] || fyagent_die "libdmg did not create a non-empty DMG"
fyagent_log "created unsigned experimental DMG: $output_path"
