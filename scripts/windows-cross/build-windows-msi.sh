#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCRIPT_PATH="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]}")"
ORIGINAL_ARGS=("$@")

ARCH_SELECTION="all"
CHECK_ONLY=0
KEEP_FAILED_WORK="${FYAGENT_WINDOWS_KEEP_FAILED_WORK:-1}"
OUTPUT_ROOT="${FYAGENT_WINDOWS_OUTPUT_ROOT:-$PROJECT_ROOT/artifacts/windows}"
WINDOWS_MSI_DATA_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/fyagent-windows-msi"
WINDOWS_MSI_CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/fyagent-windows-msi"
DEFAULT_WIX_TOOLS_DIR="$WINDOWS_MSI_DATA_ROOT/wix/WixTools314"
DEFAULT_WINE_BASE_PREFIX="$WINDOWS_MSI_DATA_ROOT/wine-prefix-base"
DEFAULT_XWIN_CACHE_DIR="$WINDOWS_MSI_CACHE_ROOT/xwin"
DEFAULT_CARGO_XWIN_VERSION="0.23.0"
DEFAULT_TAURI_CLI_VERSION="2.8.1"
DEFAULT_WINE_VERSION="11.0"

usage() {
  cat <<'EOF'
Usage: ./scripts/windows-cross/build-windows-msi.sh [options]

Build FyAgent Windows MSI candidates from an x86_64 Linux host using the
preinstalled mise + cargo-xwin + patched Tauri CLI + Wine/WiX toolchain.
The script contains no provisioning or download steps; all tools, caches, and
project dependencies must be prepared before it starts.

Options:
  --arch <all|x64|arm64>  Architectures to build (default: all).
  --output-dir <path>     Candidate output root (default: artifacts/windows).
  --check-only            Validate the complete build environment, then exit.
  --discard-failed-work   Remove the private Wine/work directory after failure.
  -h, --help              Show this help.

Environment overrides:
  TAURI_WIX_TOOLS_DIR     Host path containing verified WiX v3.14.1 binaries
                          (default: XDG data/fyagent-windows-msi/wix/WixTools314).
  WINE_BASE_PREFIX        Immutable, preinitialized Wine prefix to clone per arch
                          (default: XDG data/fyagent-windows-msi/wine-prefix-base).
                          WINEPREFIX is accepted as a compatibility fallback.
  XWIN_CACHE_DIR          Pre-populated cargo-xwin SDK/CRT cache
                          (default: XDG cache/fyagent-windows-msi/xwin).
  TAURI_CUSTOM_CLI        Patched CLI executable (default: cargo-tauri-fyagent).
  TAURI_WINE_RUNNER       Wine executable (default: first available of wine64, wine).
  TAURI_WIX_VALIDATION_MODE
                          wine-strict or native-deferred (default: native-deferred).
  FYAGENT_REQUIRE_WINDOWS_SIGNATURE
                          Set to 1 to require osslsigncode verification.
  FYAGENT_CARGO_XWIN_EXPECTED_VERSION
  FYAGENT_TAURI_CLI_EXPECTED_VERSION
  FYAGENT_WINE_EXPECTED_VERSION
                          Version substrings enforced by preflight (defaults:
                          cargo-xwin 0.23.0, Tauri CLI 2.8.1, Wine 11.0).
  FYAGENT_WINDOWS_CARGO_TARGET_DIR
                          Cargo target root (default: src-tauri/target).

Successful publication replaces only the selected version directory below the
output root, and only after every requested architecture has built and passed
the Linux-side candidate checks. Native Windows ICE and lifecycle validation
remain mandatory before release.
EOF
}

die() {
  printf '[windows-cross] error: %s\n' "$*" >&2
  return 1
}

log() {
  printf '[windows-cross] %s\n' "$*"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch)
      [[ $# -ge 2 ]] || { die "--arch requires a value"; usage >&2; exit 2; }
      ARCH_SELECTION="$2"
      shift 2
      ;;
    --arch=*)
      ARCH_SELECTION="${1#*=}"
      shift
      ;;
    --output-dir)
      [[ $# -ge 2 ]] || { die "--output-dir requires a value"; usage >&2; exit 2; }
      OUTPUT_ROOT="$2"
      shift 2
      ;;
    --output-dir=*)
      OUTPUT_ROOT="${1#*=}"
      shift
      ;;
    --check-only)
      CHECK_ONLY=1
      shift
      ;;
    --discard-failed-work)
      KEEP_FAILED_WORK=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      usage >&2
      exit 2
      ;;
  esac
done

case "$ARCH_SELECTION" in
  all) ARCHES=(x64 arm64) ;;
  x64|arm64) ARCHES=("$ARCH_SELECTION") ;;
  *) die "invalid architecture '$ARCH_SELECTION'; expected all, x64, or arm64"; exit 2 ;;
esac

case "$KEEP_FAILED_WORK" in
  0|1) ;;
  *) die "FYAGENT_WINDOWS_KEEP_FAILED_WORK must be 0 or 1"; exit 2 ;;
esac

case "$OUTPUT_ROOT" in
  /*) ;;
  *) OUTPUT_ROOT="$PROJECT_ROOT/$OUTPUT_ROOT" ;;
esac
command -v realpath >/dev/null 2>&1 || {
  printf '[windows-cross] error: required command is missing: realpath\n' >&2
  exit 1
}
OUTPUT_ROOT="$(realpath -m -- "$OUTPUT_ROOT")"
CARGO_TARGET_ROOT="${FYAGENT_WINDOWS_CARGO_TARGET_DIR:-$PROJECT_ROOT/src-tauri/target}"
case "$CARGO_TARGET_ROOT" in
  /*) ;;
  *) CARGO_TARGET_ROOT="$PROJECT_ROOT/$CARGO_TARGET_ROOT" ;;
esac
CARGO_TARGET_ROOT="$(realpath -m -- "$CARGO_TARGET_ROOT")"

case "$OUTPUT_ROOT" in
  /|"${HOME:?}"|"$PROJECT_ROOT")
    die "refusing unsafe output root: $OUTPUT_ROOT"
    exit 2
    ;;
esac
case "$CARGO_TARGET_ROOT" in
  /|"$HOME"|"$PROJECT_ROOT")
    die "refusing unsafe Cargo target root: $CARGO_TARGET_ROOT"
    exit 2
    ;;
  /mnt/*) die "Cargo target output must not use a Windows-mounted path: $CARGO_TARGET_ROOT"; exit 2 ;;
esac

require_linux_host() {
  [[ "$(uname -s)" == "Linux" ]] || die "an x86_64 Linux host is required"
  [[ "$(uname -m)" == "x86_64" ]] || die "the validated host architecture is x86_64"
  case "$PROJECT_ROOT" in
    /mnt/*) die "the project must not be built from a Windows-mounted path: $PROJECT_ROOT" ;;
  esac
}

read_mise_min_version() {
  awk -F'"' '/^[[:space:]]*min_version[[:space:]]*=/ { print $2; exit }' \
    "$PROJECT_ROOT/mise.toml"
}

version_at_least() {
  local actual="$1"
  local minimum="$2"
  [[ "$(printf '%s\n' "$minimum" "$actual" | sort -V | head -n 1)" == "$minimum" ]]
}

enter_mise_environment() {
  local mise_bin
  local mise_min_version
  local mise_version
  local mise_version_output

  require_linux_host
  command -v realpath >/dev/null 2>&1 || die "required command is missing: realpath"
  mise_bin="$(command -v mise 2>/dev/null || true)"
  [[ -n "$mise_bin" ]] || die "global mise is required; install it manually and rerun"
  mise_bin="$(realpath -e -- "$mise_bin")"
  case "$mise_bin" in
    /mnt/*) die "global mise resolves to a Windows-mounted path: $mise_bin" ;;
  esac

  mise_min_version="$(read_mise_min_version)"
  [[ -n "$mise_min_version" ]] || die "mise.toml does not declare min_version"
  mise_version_output="$("$mise_bin" --version 2>&1)"
  if [[ "$mise_version_output" =~ ([0-9]{4}\.[0-9]+\.[0-9]+) ]]; then
    mise_version="${BASH_REMATCH[1]}"
  else
    die "cannot parse mise version from: $mise_version_output"
  fi
  version_at_least "$mise_version" "$mise_min_version" || \
    die "mise $mise_min_version or newer is required; found $mise_version"

  exec env \
    FYAGENT_WINDOWS_CROSS_IN_MISE=1 \
    FYAGENT_MISE_BIN="$mise_bin" \
    "$mise_bin" --cd "$PROJECT_ROOT" exec -- \
    bash "$SCRIPT_PATH" "${ORIGINAL_ARGS[@]}"
}

if [[ "${FYAGENT_WINDOWS_CROSS_IN_MISE:-0}" != "1" ]]; then
  enter_mise_environment
fi

require_linux_host
: "${FYAGENT_MISE_BIN:?internal error: FYAGENT_MISE_BIN is missing inside mise exec}"
cd "$PROJECT_ROOT"

declare -a PREFLIGHT_ERRORS=()
declare -a REQUIRED_COMMANDS=(
  mise node pnpm python python3 rustc cargo rustup cargo-xwin
  clang lld lld-link llvm-lib llvm-rc cmake ninja pkg-config
  wineserver
  msiinfo msidump msiextract msidiff osslsigncode
  cabextract unzip zip zstd jq git file
  sha256sum realpath readlink flock find grep awk sed sort head tr date mktemp
  cp chmod mv rm rmdir touch unlink
)

record_error() {
  PREFLIGHT_ERRORS+=("$*")
}

resolved_command_path() {
  local command_name="$1"
  local command_path

  command_path="$(command -v "$command_name" 2>/dev/null || true)"
  [[ -n "$command_path" ]] || return 1
  realpath -e -- "$command_path"
}

check_required_commands() {
  local command_name
  local command_path

  for command_name in "${REQUIRED_COMMANDS[@]}"; do
    command_path="$(resolved_command_path "$command_name" 2>/dev/null || true)"
    if [[ -z "$command_path" ]]; then
      record_error "missing command: $command_name"
      continue
    fi
    case "$command_path" in
      /mnt/*) record_error "$command_name resolves to a Windows-mounted path: $command_path" ;;
    esac
  done
}

check_mise_owned_tool() {
  local command_name="$1"
  local actual
  local expected

  actual="$(resolved_command_path "$command_name" 2>/dev/null || true)"
  [[ -n "$actual" ]] || return 0
  expected="$("$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" which "$command_name" 2>/dev/null || true)"
  if [[ -z "$expected" ]]; then
    record_error "mise cannot resolve project tool: $command_name"
    return 0
  fi
  expected="$(realpath -e -- "$expected" 2>/dev/null || true)"
  [[ -n "$expected" ]] || { record_error "mise returned an invalid path for $command_name"; return 0; }
  [[ "$actual" == "$expected" ]] || \
    record_error "$command_name does not match the repository mise environment: $actual"
}

check_project_inputs() {
  local required_file
  local required_files=(
    mise.toml
    mise.lock
    package.json
    pnpm-lock.yaml
    rust-toolchain.toml
    src-tauri/Cargo.toml
    src-tauri/Cargo.lock
    src-tauri/tauri.conf.json
    src-tauri/tauri.windows.conf.json
    src-tauri/wix/per-machine-main.wxs
  )

  for required_file in "${required_files[@]}"; do
    [[ -f "$PROJECT_ROOT/$required_file" ]] || record_error "missing project input: $required_file"
  done

  [[ -d "$PROJECT_ROOT/node_modules" ]] || \
    record_error "node_modules is missing; install project dependencies manually with pnpm install --frozen-lockfile"
  [[ -x "$PROJECT_ROOT/node_modules/.bin/vite" ]] || \
    record_error "node_modules/.bin/vite is missing; project dependencies are incomplete"
}

check_rust_toolchain() {
  local arch
  local installed_components
  local installed_targets
  local target

  command -v rustup >/dev/null 2>&1 || return 0
  installed_targets="$(rustup target list --installed 2>/dev/null || true)"
  installed_components="$(rustup component list --installed 2>/dev/null || true)"

  for arch in "${ARCHES[@]}"; do
    case "$arch" in
      x64) target="x86_64-pc-windows-msvc" ;;
      arm64) target="aarch64-pc-windows-msvc" ;;
    esac
    grep -Fxq "$target" <<<"$installed_targets" || \
      record_error "Rust target is not installed in the mise toolchain: $target"
  done

  grep -Eq '^llvm-tools([[:space:]-]|$)' <<<"$installed_components" || \
    record_error "Rust component is not installed in the mise toolchain: llvm-tools"
}

CUSTOM_CLI_PATH=""
HOST_CLANG_PATH=""
WINE_RUNNER_PATH=""
WIX_TOOLS_DIR=""
WINE_BASE_PREFIX_RESOLVED=""
XWIN_CACHE_DIR_RESOLVED=""

check_custom_tauri_cli() {
  local cli_name="${TAURI_CUSTOM_CLI:-cargo-tauri-fyagent}"
  local cli_build_help
  local cli_version
  local expected_version="${FYAGENT_TAURI_CLI_EXPECTED_VERSION:-$DEFAULT_TAURI_CLI_VERSION}"

  CUSTOM_CLI_PATH="$(resolved_command_path "$cli_name" 2>/dev/null || true)"
  if [[ -z "$CUSTOM_CLI_PATH" ]]; then
    record_error "missing patched Tauri CLI: $cli_name"
    return 0
  fi
  case "$CUSTOM_CLI_PATH" in
    *node_modules*)
      record_error "patched Tauri CLI must not resolve from node_modules: $CUSTOM_CLI_PATH"
      return 0
      ;;
    /mnt/*)
      record_error "patched Tauri CLI resolves to a Windows-mounted path: $CUSTOM_CLI_PATH"
      return 0
      ;;
  esac

  cli_version="$("$CUSTOM_CLI_PATH" --version 2>&1 || true)"
  [[ -n "$cli_version" ]] || record_error "patched Tauri CLI did not report a version"
  if [[ "$cli_version" != *"$expected_version"* ]]; then
    record_error "patched Tauri CLI version mismatch: expected '$expected_version', got '$cli_version'"
  fi

  cli_build_help="$("$CUSTOM_CLI_PATH" build --help 2>&1 || true)"
  if ! grep -Eq 'possible values:.*[[:space:],]msi([[:space:],]|])' <<<"$cli_build_help"; then
    record_error "patched Tauri CLI does not expose the Linux MSI bundle capability"
  fi
}

check_clang_drivers() {
  local clang_cl_version

  HOST_CLANG_PATH="$(resolved_command_path clang 2>/dev/null || true)"
  [[ -n "$HOST_CLANG_PATH" ]] || return 0
  clang_cl_version="$("$HOST_CLANG_PATH" --driver-mode=cl --version 2>&1 || true)"
  if [[ "$clang_cl_version" != *"clang version"* ]]; then
    record_error "clang does not support the CL-compatible driver mode required by cargo-xwin"
  fi
}

check_wine_runner() {
  local runner_input="${TAURI_WINE_RUNNER:-}"
  local runner_candidate

  if [[ -n "$runner_input" ]]; then
    WINE_RUNNER_PATH="$(resolved_command_path "$runner_input" 2>/dev/null || true)"
    if [[ -z "$WINE_RUNNER_PATH" ]]; then
      record_error "TAURI_WINE_RUNNER is not an executable command: $runner_input"
      return 0
    fi
  else
    for runner_candidate in wine64 wine; do
      WINE_RUNNER_PATH="$(resolved_command_path "$runner_candidate" 2>/dev/null || true)"
      [[ -n "$WINE_RUNNER_PATH" ]] && break
    done
    if [[ -z "$WINE_RUNNER_PATH" ]]; then
      record_error "missing Wine runner: install wine or set TAURI_WINE_RUNNER"
      return 0
    fi
  fi

  case "$WINE_RUNNER_PATH" in
    /mnt/*) record_error "Wine runner resolves to a Windows-mounted path: $WINE_RUNNER_PATH" ;;
  esac
}

check_optional_tool_versions() {
  local actual
  local cargo_xwin_expected="${FYAGENT_CARGO_XWIN_EXPECTED_VERSION:-$DEFAULT_CARGO_XWIN_VERSION}"
  local wine_expected="${FYAGENT_WINE_EXPECTED_VERSION:-$DEFAULT_WINE_VERSION}"

  if command -v cargo-xwin >/dev/null 2>&1; then
    actual="$(cargo-xwin --version 2>&1 || true)"
    [[ "$actual" == *"$cargo_xwin_expected"* ]] || \
      record_error "cargo-xwin version mismatch: expected '$cargo_xwin_expected', got '$actual'"
  fi

  if [[ -n "$WINE_RUNNER_PATH" ]]; then
    actual="$("$WINE_RUNNER_PATH" --version 2>&1 || true)"
    [[ "$actual" == *"$wine_expected"* ]] || \
      record_error "Wine version mismatch: expected '$wine_expected', got '$actual'"
  fi
}

check_wix_tools() {
  local required_wix_file
  local wix_input="${TAURI_WIX_TOOLS_DIR:-$DEFAULT_WIX_TOOLS_DIR}"

  WIX_TOOLS_DIR="$(realpath -e -- "$wix_input" 2>/dev/null || true)"
  if [[ -z "$WIX_TOOLS_DIR" || ! -d "$WIX_TOOLS_DIR" ]]; then
    record_error "TAURI_WIX_TOOLS_DIR does not exist: $wix_input"
    return 0
  fi
  case "$WIX_TOOLS_DIR" in
    /mnt/*) record_error "WiX tools must not resolve from a Windows-mounted path: $WIX_TOOLS_DIR" ;;
  esac

  for required_wix_file in \
    candle.exe candle.exe.config darice.cub light.exe light.exe.config \
    wconsole.dll winterop.dll wix.dll WixUIExtension.dll WixUtilExtension.dll; do
    [[ -f "$WIX_TOOLS_DIR/$required_wix_file" ]] || \
      record_error "missing WiX tool: $WIX_TOOLS_DIR/$required_wix_file"
  done
}

check_wine_prefix() {
  local prefix_input="${WINE_BASE_PREFIX:-${WINEPREFIX:-$DEFAULT_WINE_BASE_PREFIX}}"

  WINE_BASE_PREFIX_RESOLVED="$(realpath -e -- "$prefix_input" 2>/dev/null || true)"
  if [[ -z "$WINE_BASE_PREFIX_RESOLVED" || ! -d "$WINE_BASE_PREFIX_RESOLVED" ]]; then
    record_error "Wine base prefix does not exist: $prefix_input"
    return 0
  fi
  case "$WINE_BASE_PREFIX_RESOLVED" in
    /mnt/*) record_error "Wine base prefix must not be stored on a Windows-mounted path: $WINE_BASE_PREFIX_RESOLVED" ;;
  esac
  [[ -d "$WINE_BASE_PREFIX_RESOLVED/drive_c" ]] || \
    record_error "Wine base prefix is not initialized: $WINE_BASE_PREFIX_RESOLVED/drive_c is missing"
  [[ -r "$WINE_BASE_PREFIX_RESOLVED/system.reg" ]] || \
    record_error "Wine base prefix is incomplete: system.reg is missing or unreadable"
}

check_xwin_cache() {
  local cache_input="${XWIN_CACHE_DIR:-$DEFAULT_XWIN_CACHE_DIR}"

  XWIN_CACHE_DIR_RESOLVED="$(realpath -e -- "$cache_input" 2>/dev/null || true)"
  if [[ -z "$XWIN_CACHE_DIR_RESOLVED" || ! -d "$XWIN_CACHE_DIR_RESOLVED" ]]; then
    record_error "XWIN_CACHE_DIR does not exist: $cache_input"
    return 0
  fi
  case "$XWIN_CACHE_DIR_RESOLVED" in
    /mnt/*) record_error "cargo-xwin cache must not be stored on a Windows-mounted path: $XWIN_CACHE_DIR_RESOLVED" ;;
  esac
  if ! find "$XWIN_CACHE_DIR_RESOLVED" -mindepth 1 -print -quit 2>/dev/null | grep -q .; then
    record_error "XWIN_CACHE_DIR is empty; prefetch and verify the Windows SDK/CRT manually"
  fi
}

check_build_policy() {
  local backend="${TAURI_WIX_BACKEND:-wine}"
  local keep_intermediates="${TAURI_WIX_KEEP_INTERMEDIATES:-1}"
  local validation_mode="${TAURI_WIX_VALIDATION_MODE:-native-deferred}"

  [[ "$backend" == "wine" ]] || record_error "TAURI_WIX_BACKEND must be wine"
  [[ "$keep_intermediates" == "1" ]] || record_error "TAURI_WIX_KEEP_INTERMEDIATES must be 1"
  case "$validation_mode" in
    wine-strict|native-deferred) ;;
    *) record_error "TAURI_WIX_VALIDATION_MODE must be wine-strict or native-deferred" ;;
  esac
  case "${FYAGENT_REQUIRE_WINDOWS_SIGNATURE:-0}" in
    0|1) ;;
    *) record_error "FYAGENT_REQUIRE_WINDOWS_SIGNATURE must be 0 or 1" ;;
  esac
}

run_preflight() {
  local managed_tool

  check_required_commands
  check_project_inputs
  for managed_tool in node pnpm python rustc cargo; do
    check_mise_owned_tool "$managed_tool"
  done
  check_rust_toolchain
  check_custom_tauri_cli
  check_clang_drivers
  check_wine_runner
  check_optional_tool_versions
  check_wix_tools
  check_wine_prefix
  check_xwin_cache
  check_build_policy

  if (( ${#PREFLIGHT_ERRORS[@]} > 0 )); then
    printf '[windows-cross] preflight failed with %d problem(s):\n' "${#PREFLIGHT_ERRORS[@]}" >&2
    printf '  - %s\n' "${PREFLIGHT_ERRORS[@]}" >&2
    return 1
  fi

  log "preflight passed"
  log "mise: $("$FYAGENT_MISE_BIN" --version 2>&1 | head -n 1)"
  log "cargo-xwin: $(cargo-xwin --version 2>&1 | head -n 1)"
  log "Tauri CLI: $("$CUSTOM_CLI_PATH" --version 2>&1 | head -n 1)"
  log "Wine: $("$WINE_RUNNER_PATH" --version 2>&1 | head -n 1) ($WINE_RUNNER_PATH)"
  log "WiX tools: $WIX_TOOLS_DIR"
  log "xwin cache: $XWIN_CACHE_DIR_RESOLVED"
  log "Wine base prefix: $WINE_BASE_PREFIX_RESOLVED"
}

run_preflight

if [[ "$CHECK_ONLY" == "1" ]]; then
  log "environment check completed; no build was started"
  exit 0
fi

mapfile -t PROJECT_METADATA < <(
  node -e '
    const fs = require("node:fs");
    const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
    process.stdout.write(
      String(config.productName ?? "") + "\n" + String(config.version ?? "") + "\n",
    );
  '
)
[[ "${#PROJECT_METADATA[@]}" -eq 2 ]] || die "cannot read product metadata from src-tauri/tauri.conf.json"
PRODUCT_NAME="${PROJECT_METADATA[0]}"
PRODUCT_VERSION="${PROJECT_METADATA[1]}"
[[ "$PRODUCT_NAME" == "FyAgent" ]] || die "unexpected productName: $PRODUCT_NAME"
[[ "$PRODUCT_VERSION" =~ ^[0-9A-Za-z][0-9A-Za-z._+-]*$ ]] || \
  die "unsafe product version for artifact paths: $PRODUCT_VERSION"

STATE_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/fyagent-windows-cross"
WORK_ROOT="${FYAGENT_WINDOWS_WORK_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/fyagent-windows-cross/work}"
STATE_ROOT="$(realpath -m -- "$STATE_ROOT")"
WORK_ROOT="$(realpath -m -- "$WORK_ROOT")"
case "$WORK_ROOT" in
  /|"$HOME"|"$PROJECT_ROOT") die "refusing unsafe work root: $WORK_ROOT" ;;
esac

mkdir -p "$STATE_ROOT" "$WORK_ROOT"
exec 9>"$STATE_ROOT/build.lock"
flock -n 9 || die "another FyAgent Windows MSI cross-build is already running"

JOB_ROOT="$(mktemp -d "$WORK_ROOT/build.XXXXXX")"
PUBLISH_ROOT="$JOB_ROOT/publish/$PRODUCT_VERSION"
RELEASE_STAGE=""
ACTIVE_WINE_PREFIX=""
ACTIVE_XWIN_CLANG_WRAPPER=""
FINAL_VERSION_DIR=""
BACKUP_VERSION_DIR=""
mkdir -p "$PUBLISH_ROOT"

assert_owned_path() {
  local candidate="$1"
  local allowed_root="$2"
  local normalized_candidate
  local normalized_root

  normalized_candidate="$(realpath -m -- "$candidate")"
  normalized_root="$(realpath -m -- "$allowed_root")"
  [[ "$normalized_candidate" != "$normalized_root" ]] || \
    die "refusing to operate on owned root itself: $normalized_root"
  case "$normalized_candidate" in
    "$normalized_root"/*) ;;
    *) die "refusing to operate outside $normalized_root: $normalized_candidate" ;;
  esac
}

remove_owned_tree() {
  local candidate="$1"
  local allowed_root="$2"
  [[ -e "$candidate" ]] || return 0
  assert_owned_path "$candidate" "$allowed_root"
  rm -rf -- "$candidate"
}

stop_active_wineserver() {
  if [[ -n "$ACTIVE_WINE_PREFIX" && -d "$ACTIVE_WINE_PREFIX" ]]; then
    WINEPREFIX="$ACTIVE_WINE_PREFIX" wineserver -k >/dev/null 2>&1 || true
  fi
  ACTIVE_WINE_PREFIX=""
}

cleanup_active_xwin_clang_link() {
  local clang_link="$XWIN_CACHE_DIR_RESOLVED/clang-cl"
  local link_target

  if [[ -n "$ACTIVE_XWIN_CLANG_WRAPPER" && -L "$clang_link" ]]; then
    link_target="$(readlink -- "$clang_link")"
    if [[ "$link_target" == "$ACTIVE_XWIN_CLANG_WRAPPER" ]]; then
      unlink "$clang_link"
    fi
  fi
  ACTIVE_XWIN_CLANG_WRAPPER=""
}

on_exit() {
  local status=$?
  trap - EXIT INT TERM

  cleanup_active_xwin_clang_link || true
  stop_active_wineserver
  if [[ "$status" != "0" && -n "$BACKUP_VERSION_DIR" && \
        -e "$BACKUP_VERSION_DIR" && -n "$FINAL_VERSION_DIR" && \
        ! -e "$FINAL_VERSION_DIR" ]]; then
    mv -- "$BACKUP_VERSION_DIR" "$FINAL_VERSION_DIR" || true
  fi
  if [[ -n "$RELEASE_STAGE" && -e "$RELEASE_STAGE" ]]; then
    remove_owned_tree "$RELEASE_STAGE" "$OUTPUT_ROOT" || true
  fi

  if [[ -n "$JOB_ROOT" && -e "$JOB_ROOT" ]]; then
    if [[ "$status" == "0" || "$KEEP_FAILED_WORK" == "0" ]]; then
      remove_owned_tree "$JOB_ROOT" "$WORK_ROOT" || true
    else
      printf '[windows-cross] failed work retained for diagnosis: %s\n' "$JOB_ROOT" >&2
    fi
  fi
  exit "$status"
}
trap on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

clone_wine_prefix() {
  local arch="$1"
  local prefix="$JOB_ROOT/wine/$arch/prefix"

  mkdir -p "$prefix"
  if ! cp -a --reflink=auto "$WINE_BASE_PREFIX_RESOLVED/." "$prefix/" 2>/dev/null; then
    remove_owned_tree "$prefix" "$JOB_ROOT"
    mkdir -p "$prefix"
    cp -a "$WINE_BASE_PREFIX_RESOLVED/." "$prefix/"
  fi
  chmod -R u+w "$prefix"
  printf '%s\n' "$prefix"
}

create_xwin_compiler_wrappers() {
  local arch="$1"
  local wrapper_dir="$JOB_ROOT/tool-wrappers/$arch"

  mkdir -p "$wrapper_dir"
  if [[ "$arch" == "arm64" ]]; then
    # ring 0.17 forces the literal `clang` driver for Windows ARM64 while
    # cargo-xwin supplies CL-style include arguments. The same LLVM binary's
    # documented CL mode accepts both ring's C and ARM64 assembly inputs.
    printf '#!/usr/bin/env bash\nexec %q --driver-mode=cl "$@"\n' "$HOST_CLANG_PATH" \
      >"$wrapper_dir/clang"
    chmod 0755 "$wrapper_dir/clang"
  fi
  printf '%s\n' "$wrapper_dir"
}

target_for_arch() {
  case "$1" in
    x64) printf '%s\n' 'x86_64-pc-windows-msvc' ;;
    arm64) printf '%s\n' 'aarch64-pc-windows-msvc' ;;
    *) die "unsupported architecture: $1" ;;
  esac
}

artifact_name_for_arch() {
  case "$1" in
    x64) printf 'FyAgent-%s-Windows.msi\n' "$PRODUCT_VERSION" ;;
    arm64) printf 'FyAgent-%s-Windows-arm64.msi\n' "$PRODUCT_VERSION" ;;
    *) die "unsupported architecture: $1" ;;
  esac
}

verify_linux_candidate() {
  local msi="$1"
  local arch="$2"
  local evidence_dir="$3"
  local listing
  local rendered_file
  local verify_dir="$JOB_ROOT/verify/$arch"
  local -a rendered_wxs=()
  local table
  local tables=(
    Property Directory Component Feature FeatureComponents File Registry
    Shortcut Upgrade InstallExecuteSequence
  )

  mkdir -p "$verify_dir"
  for table in "${tables[@]}"; do
    msiinfo export "$msi" "$table" >"$verify_dir/$table.idt" || \
      die "$arch MSI is missing or cannot export required table: $table"
  done

  grep -Fq $'ProductName\tFyAgent' "$verify_dir/Property.idt" || \
    die "$arch MSI ProductName contract failed"
  if ! grep -Fq $'ARPNOREPAIR\t1' "$verify_dir/Property.idt" && \
     ! grep -Fq $'ARPNOREPAIR\tyes' "$verify_dir/Property.idt"; then
    die "$arch MSI ARPNOREPAIR contract failed"
  fi
  grep -Fiq 'fyagent' "$verify_dir/Registry.idt" || \
    die "$arch MSI does not contain the fyagent protocol registry contract"

  listing="$(msiextract -l "$msi")" || die "$arch MSI payload cannot be listed"
  grep -Fiq 'fyagent.exe' <<<"$listing" || die "$arch MSI payload does not contain fyagent.exe"

  if grep -R -a -E '/home/|/workspace/|/mnt/' "$verify_dir" >/dev/null 2>&1 || \
     grep -R -a -F "$PROJECT_ROOT/" "$verify_dir" >/dev/null 2>&1; then
    die "$arch MSI tables contain a Linux host path"
  fi

  mapfile -d '' -t rendered_wxs < <(find "$evidence_dir" -type f -name '*.wxs' -print0)
  (( ${#rendered_wxs[@]} > 0 )) || \
    die "$arch build did not retain rendered WXS evidence in $evidence_dir"
  for rendered_file in "${rendered_wxs[@]}"; do
    if grep -a -E '/home/|/workspace/|/mnt/' "$rendered_file" >/dev/null 2>&1 || \
       grep -a -F "$PROJECT_ROOT/" "$rendered_file" >/dev/null 2>&1; then
      die "$arch rendered WXS contains a Linux host path: $rendered_file"
    fi
  done

  if [[ "${FYAGENT_REQUIRE_WINDOWS_SIGNATURE:-0}" == "1" ]]; then
    osslsigncode verify -in "$msi" >"$verify_dir/signature.txt" 2>&1 || {
      cat "$verify_dir/signature.txt" >&2
      die "$arch MSI signature verification failed"
    }
  fi
}

write_native_validation_request() {
  local artifact_name="$1"
  local artifact_sha256="$2"
  local arch="$3"
  local output="$4"
  local created_at

  created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  node - "$artifact_name" "$artifact_sha256" "$arch" "$created_at" >"$output" <<'NODE'
const [artifact, artifactSha256, architecture, createdAt] = process.argv.slice(2);
const request = {
  schema: "fyagent-native-validation-request/v1",
  required: true,
  reason: "Windows Installer ICE and lifecycle validation are deferred from the Wine build",
  artifact,
  artifactSha256,
  architecture,
  iceCub: "darice.cub",
  policy: "fyagent-msi-native/v1",
  createdAt,
};
process.stdout.write(`${JSON.stringify(request, null, 2)}\n`);
NODE
}

declare -a PUBLISHED_MSI_PATHS=()

build_architecture() {
  local arch="$1"
  local target
  local prefix
  local evidence_dir
  local bundle_dir
  local build_marker
  local compiler_wrapper_dir
  local artifact_name
  local arch_publish_dir
  local artifact_sha256
  local -a generated_msis=()

  target="$(target_for_arch "$arch")"
  artifact_name="$(artifact_name_for_arch "$arch")"
  prefix="$(clone_wine_prefix "$arch")"
  compiler_wrapper_dir="$(create_xwin_compiler_wrappers "$arch")"
  if [[ "$arch" == "arm64" ]]; then
    ACTIVE_XWIN_CLANG_WRAPPER="$compiler_wrapper_dir/clang"
  fi
  ACTIVE_WINE_PREFIX="$prefix"
  evidence_dir="$JOB_ROOT/evidence/$arch"
  bundle_dir="$CARGO_TARGET_ROOT/$target/release/bundle/msi"
  build_marker="$JOB_ROOT/build-started-$arch"
  mkdir -p "$evidence_dir"
  touch "$build_marker"

  log "building $arch MSI for $target"
  if ! (
    export CI=true
    export CARGO_NET_OFFLINE=true
    export CARGO_TARGET_DIR="$CARGO_TARGET_ROOT"
    export XWIN_CACHE_DIR="$XWIN_CACHE_DIR_RESOLVED"
    export XWIN_CROSS_COMPILER=clang-cl
    export PATH="$compiler_wrapper_dir:$PATH"
    export WINEPREFIX="$prefix"
    export WINEARCH=win64
    export WINEDEBUG="${WINEDEBUG:--all}"
    export TAURI_WINE_RUNNER="$WINE_RUNNER_PATH"
    export TAURI_WIX_BACKEND=wine
    export TAURI_WIX_TOOLS_DIR="$WIX_TOOLS_DIR"
    export TAURI_WIX_VALIDATION_MODE="${TAURI_WIX_VALIDATION_MODE:-native-deferred}"
    export TAURI_WIX_KEEP_INTERMEDIATES=1
    export TAURI_WIX_EVIDENCE_DIR="$evidence_dir"
    export TZ=UTC
    export LC_ALL=C.UTF-8

    "$CUSTOM_CLI_PATH" build \
      --runner cargo-xwin \
      --target "$target" \
      --bundles msi
  ); then
    cleanup_active_xwin_clang_link
    stop_active_wineserver
    die "$arch MSI build failed"
  fi
  cleanup_active_xwin_clang_link
  stop_active_wineserver

  [[ -d "$bundle_dir" ]] || die "$arch MSI bundle directory was not created: $bundle_dir"
  mapfile -d '' -t generated_msis < <(
    find "$bundle_dir" -maxdepth 1 -type f -name '*.msi' -newer "$build_marker" -print0
  )
  [[ "${#generated_msis[@]}" -eq 1 ]] || \
    die "expected exactly one newly generated $arch MSI, found ${#generated_msis[@]}"

  verify_linux_candidate "${generated_msis[0]}" "$arch" "$evidence_dir"

  arch_publish_dir="$PUBLISH_ROOT/$arch"
  mkdir -p "$arch_publish_dir/evidence"
  cp -p -- "${generated_msis[0]}" "$arch_publish_dir/$artifact_name"
  cp -a -- "$evidence_dir/." "$arch_publish_dir/evidence/"

  artifact_sha256="$(sha256sum "$arch_publish_dir/$artifact_name" | awk '{print $1}')"
  printf '%s  %s\n' "$artifact_sha256" "$artifact_name" \
    >"$arch_publish_dir/$artifact_name.sha256"
  write_native_validation_request \
    "$artifact_name" "$artifact_sha256" "$arch" \
    "$arch_publish_dir/native-validation-required.json"
  (
    cd "$arch_publish_dir"
    sha256sum -c "$artifact_name.sha256"
  )

  PUBLISHED_MSI_PATHS+=("$arch/$artifact_name")
  log "$arch candidate prepared: $arch_publish_dir/$artifact_name"
}

for arch in "${ARCHES[@]}"; do
  build_architecture "$arch"
done

CHECKSUMS_FILE="$PUBLISH_ROOT/checksums.txt"
: >"$CHECKSUMS_FILE"
for relative_msi in "${PUBLISHED_MSI_PATHS[@]}"; do
  printf '%s  %s\n' \
    "$(sha256sum "$PUBLISH_ROOT/$relative_msi" | awk '{print $1}')" \
    "$relative_msi" >>"$CHECKSUMS_FILE"
done
(
  cd "$PUBLISH_ROOT"
  sha256sum -c checksums.txt
)

mkdir -p "$OUTPUT_ROOT"
RELEASE_STAGE="$(mktemp -d "$OUTPUT_ROOT/.publish.XXXXXX")"
mkdir -p "$RELEASE_STAGE/$PRODUCT_VERSION"
cp -a -- "$PUBLISH_ROOT/." "$RELEASE_STAGE/$PRODUCT_VERSION/"
(
  cd "$RELEASE_STAGE/$PRODUCT_VERSION"
  sha256sum -c checksums.txt
)

FINAL_VERSION_DIR="$OUTPUT_ROOT/$PRODUCT_VERSION"
if [[ -e "$FINAL_VERSION_DIR" ]]; then
  BACKUP_VERSION_DIR="$OUTPUT_ROOT/.backup.$PRODUCT_VERSION.$BASHPID"
  [[ ! -e "$BACKUP_VERSION_DIR" ]] || die "publication backup path already exists: $BACKUP_VERSION_DIR"
  mv -- "$FINAL_VERSION_DIR" "$BACKUP_VERSION_DIR"
fi

if ! mv -- "$RELEASE_STAGE/$PRODUCT_VERSION" "$FINAL_VERSION_DIR"; then
  if [[ -n "$BACKUP_VERSION_DIR" && -e "$BACKUP_VERSION_DIR" ]]; then
    mv -- "$BACKUP_VERSION_DIR" "$FINAL_VERSION_DIR" || true
  fi
  die "failed to publish Windows MSI candidates"
fi
rmdir "$RELEASE_STAGE"
RELEASE_STAGE=""
if [[ -n "$BACKUP_VERSION_DIR" && -e "$BACKUP_VERSION_DIR" ]]; then
  remove_owned_tree "$BACKUP_VERSION_DIR" "$OUTPUT_ROOT"
fi
BACKUP_VERSION_DIR=""

cat <<EOF

FyAgent Linux-to-Windows MSI build completed.
  Candidates: $FINAL_VERSION_DIR
  Checksums:  $FINAL_VERSION_DIR/checksums.txt

The MSI files passed Linux-side structural and path checks. Native Windows ICE,
installation, launch, deep-link, upgrade, uninstall, and signature acceptance
remain pending and must validate these exact SHA-256 values before release.
EOF
