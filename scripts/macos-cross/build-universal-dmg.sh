#!/usr/bin/env bash
set -euo pipefail
# shellcheck source-path=SCRIPTDIR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

usage() {
  cat <<EOF
Usage: ./scripts/macos-cross/build-universal-dmg.sh [--accept-risk]

Build one FyAgent macOS Universal DMG from WSL2 Ubuntu 22.04/24.04 x86_64.
Global mise >= $FYAGENT_MISE_MIN_VERSION must already be installed. The first
run installs missing apt packages, runs "mise install" for repository-pinned
development tools, and stores macOS cross-build inputs under:
  cache: $FYAGENT_MACOS_CROSS_CACHE_ROOT
  data:  $FYAGENT_MACOS_CROSS_DATA_ROOT
  state: $FYAGENT_MACOS_CROSS_STATE_ROOT

Options:
  --accept-risk  Record the fixed third-party SDK/GPL/experimental-DMG risk
                 acknowledgement without an interactive prompt.
  --help         Show this help.

Only universal-apple-darwin is supported. The output is ad-hoc signed,
unnotarized, experimental, and not suitable for public release.
Successful local artifacts (DMG, SHA-256, and manifest) are published under:
  dist-bundle/macos/
EOF
}

accept_risk=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --accept-risk) accept_risk=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fyagent_die "unknown argument: $1"; usage >&2; exit 2 ;;
  esac
done

global_mise_bin="$(command -v mise 2>/dev/null || true)"
[[ -n "$global_mise_bin" ]] || fyagent_die "global mise is required; install mise and rerun"
global_mise_bin="$(realpath -e -- "$global_mise_bin")"
case "$global_mise_bin" in
  /mnt/*) fyagent_die "global mise resolves to a Windows-mounted path: $global_mise_bin" ;;
esac
export FYAGENT_MISE_BIN="$global_mise_bin"

# Keep Windows interop tools and unrelated user runtime managers out of the build.
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

mkdir -p "$FYAGENT_MACOS_CROSS_STATE_ROOT"
exec 9>"$FYAGENT_MACOS_CROSS_STATE_ROOT/build.lock"
flock -n 9 || fyagent_die "another FyAgent macOS cross-build is already running"

bootstrap_args=()
if [[ "$accept_risk" == "1" ]]; then
  bootstrap_args+=(--accept-risk)
fi

fyagent_log "stage 1/3: validating global mise and bootstrapping the WSL host"
"$SCRIPT_DIR/bootstrap-host.sh" "${bootstrap_args[@]}"

fyagent_log "stage 2/3: provisioning repository-pinned tools through global mise and macOS tools"
"$SCRIPT_DIR/provision-toolchains.sh"

fyagent_log "stage 3/3: building and validating the Universal DMG"
"$FYAGENT_MISE_BIN" trust --yes "$PROJECT_ROOT/mise.toml" >/dev/null
"$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- bash "$SCRIPT_DIR/build-package.sh"
