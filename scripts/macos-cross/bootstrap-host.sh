#!/usr/bin/env bash
set -euo pipefail
# shellcheck source-path=SCRIPTDIR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

readonly REQUIRED_APT_PACKAGES=(
  build-essential ca-certificates clang llvm lld cmake ninja-build
  python3 git make patch perl sed tar gzip xz-utils bzip2 cpio curl
  pkg-config rsync jq file unzip shellcheck
  libxml2-dev libssl-dev zlib1g-dev liblzma-dev libbz2-dev uuid-dev
  xorriso
)

usage() {
  cat <<'USAGE'
Usage: bootstrap-host.sh [--accept-risk]

Internal helper for build-universal-dmg.sh. It validates the WSL host, installs
missing Ubuntu packages, records the fixed-source risk acknowledgement, and
validates the user-installed global mise binary.
USAGE
}

check_host() {
  local distro_id
  local distro_version
  local filesystem_type
  local kernel_release

  [[ "$(uname -s)" == "Linux" ]] || fyagent_die "this workflow must run inside WSL2/Linux"
  [[ "$(uname -m)" == "x86_64" ]] || fyagent_die "only x86_64 WSL hosts are supported"
  [[ -r /proc/sys/kernel/osrelease ]] || fyagent_die "cannot inspect the Linux kernel release"
  kernel_release="$(tr '[:upper:]' '[:lower:]' </proc/sys/kernel/osrelease)"
  [[ "$kernel_release" == *microsoft* && "$kernel_release" == *wsl2* ]] || \
    fyagent_die "this workflow requires WSL2 (kernel: $kernel_release)"

  [[ -r /etc/os-release ]] || fyagent_die "/etc/os-release is missing"
  # shellcheck disable=SC1091
  source /etc/os-release
  distro_id="${ID:-}"
  distro_version="${VERSION_ID:-}"
  [[ "$distro_id" == "ubuntu" ]] || fyagent_die "only Ubuntu WSL is supported (found ${PRETTY_NAME:-unknown})"
  case "$distro_version" in
    22.04|24.04) ;;
    *) fyagent_die "only Ubuntu 22.04 and 24.04 are supported (found $distro_version)" ;;
  esac

  case "$PROJECT_ROOT" in
    /mnt/[a-zA-Z]|/mnt/[a-zA-Z]/*)
      fyagent_die "move the repository to the WSL ext4 filesystem; DrvFS paths under /mnt/<drive> are unsupported"
      ;;
  esac
  filesystem_type="$(findmnt -T "$PROJECT_ROOT" -n -o FSTYPE 2>/dev/null || true)"
  case "$filesystem_type" in
    drvfs|9p)
      fyagent_die "repository filesystem $filesystem_type is unsupported; use the WSL ext4 filesystem"
      ;;
  esac

  fyagent_log "host accepted: Ubuntu $distro_version, WSL2 x86_64, filesystem ${filesystem_type:-unknown}"
}

install_apt_dependencies() {
  local missing=()
  local package_name
  local apt_prefix=()

  for package_name in "${REQUIRED_APT_PACKAGES[@]}"; do
    if ! dpkg-query -W -f='${Status}' "$package_name" 2>/dev/null | grep -qx 'install ok installed'; then
      missing+=("$package_name")
    fi
  done

  if [[ "${#missing[@]}" -eq 0 ]]; then
    fyagent_log "all Ubuntu build dependencies are already installed"
    return 0
  fi

  fyagent_log "installing missing Ubuntu packages: ${missing[*]}"
  if [[ "$(id -u)" -ne 0 ]]; then
    fyagent_require_command sudo
    sudo -v
    apt_prefix=(sudo)
  fi
  "${apt_prefix[@]}" apt-get update
  "${apt_prefix[@]}" env DEBIAN_FRONTEND=noninteractive apt-get install -y "${missing[@]}"
}

record_apt_versions() {
  local output_dir="${FYAGENT_MACOS_CROSS_STATE_ROOT}/host"
  local temporary_file
  local package_name

  mkdir -p "$output_dir"
  temporary_file="$(mktemp "${output_dir}/apt-packages.XXXXXX")"
  for package_name in "${REQUIRED_APT_PACKAGES[@]}"; do
    dpkg-query -W -f='${Package}=${Version}\n' "$package_name" >>"$temporary_file"
  done
  sort -o "$temporary_file" "$temporary_file"
  mv -f -- "$temporary_file" "$output_dir/apt-packages.txt"
}

risk_fingerprint() {
  printf '%s\n' \
    "$FYAGENT_MACOS_SDK_URL" \
    "$FYAGENT_MACOS_SDK_SHA256" \
    "$FYAGENT_OSXCROSS_COMMIT" \
    "$FYAGENT_LIBDMG_COMMIT" | sha256sum | awk '{print $1}'
}

acknowledge_risks() {
  local accept_risk="$1"
  local marker_dir="${FYAGENT_MACOS_CROSS_STATE_ROOT}/risk-acceptance"
  local marker_path
  local answer

  marker_path="${marker_dir}/$(risk_fingerprint)"

  if [[ -f "$marker_path" ]]; then
    fyagent_log "fixed-source risk acknowledgement is current"
    return 0
  fi

  cat >&2 <<'NOTICE'

This workflow downloads a third-party copy of the macOS 14.5 SDK and uses it
with OSXCross on non-Apple hardware. The pinned checksum verifies the selected
third-party release; it does not establish Apple authorization or resolve
license restrictions. DMG creation uses GPL-3.0 libdmg-hfsplus, which upstream
describes as highly experimental. The resulting ad-hoc, unnotarized DMG is for
internal testing only and still requires acceptance on a real Mac.
NOTICE

  if [[ "$accept_risk" != "1" ]]; then
    if [[ ! -t 0 ]]; then
      fyagent_die "risk acknowledgement is required; rerun with --accept-risk in non-interactive environments"
      return 1
    fi
    read -r -p "Type ACCEPT to continue: " answer
    [[ "$answer" == "ACCEPT" ]] || fyagent_die "risk acknowledgement was not accepted"
  fi

  umask 077
  mkdir -p "$marker_dir"
  {
    printf 'accepted_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'sdk_sha256=%s\n' "$FYAGENT_MACOS_SDK_SHA256"
    printf 'osxcross_commit=%s\n' "$FYAGENT_OSXCROSS_COMMIT"
    printf 'libdmg_commit=%s\n' "$FYAGENT_LIBDMG_COMMIT"
  } >"$marker_path"
  fyagent_log "recorded fixed-source risk acknowledgement"
}

validate_global_mise() {
  local actual_version
  local lowest_version
  local version_output

  [[ -n "${FYAGENT_MISE_BIN:-}" && -x "$FYAGENT_MISE_BIN" ]] || \
    fyagent_die "global mise is required; install mise and rerun"
  case "$FYAGENT_MISE_BIN" in
    /mnt/*) fyagent_die "global mise must not resolve under /mnt: $FYAGENT_MISE_BIN" ;;
  esac

  version_output="$(fyagent_version_output "$FYAGENT_MISE_BIN" --version)"
  actual_version="${version_output#mise }"
  actual_version="${actual_version%% *}"
  [[ "$actual_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || \
    fyagent_die "cannot parse global mise version: $version_output"
  lowest_version="$(printf '%s\n%s\n' "$FYAGENT_MISE_MIN_VERSION" "$actual_version" | sort -V | head -n 1)"
  [[ "$lowest_version" == "$FYAGENT_MISE_MIN_VERSION" ]] || \
    fyagent_die "global mise $actual_version is too old; require >= $FYAGENT_MISE_MIN_VERSION"
  fyagent_log "global mise accepted: $actual_version at $FYAGENT_MISE_BIN"
}

accept_risk=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --accept-risk) accept_risk=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fyagent_die "unknown argument: $1"; usage >&2; exit 2 ;;
  esac
done

check_host
validate_global_mise
install_apt_dependencies
record_apt_versions
acknowledge_risks "$accept_risk"
