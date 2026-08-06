#!/usr/bin/env bash
# Central source of truth for the WSL macOS cross-build supply chain.
# shellcheck disable=SC2034

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "error: source this file instead of executing it" >&2
  exit 2
fi

if [[ "${FYAGENT_MACOS_CROSS_CONSTANTS_LOADED:-0}" == "1" ]]; then
  return 0
fi
readonly FYAGENT_MACOS_CROSS_CONSTANTS_LOADED=1

readonly FYAGENT_MISE_MIN_VERSION="2026.8.0"

readonly FYAGENT_MACOS_SDK_VERSION="14.5"
readonly FYAGENT_MACOS_SDK_ARCHIVE="MacOSX14.5.sdk.tar.xz"
readonly FYAGENT_MACOS_SDK_URL="https://github.com/joseluisq/macosx-sdks/releases/download/14.5/${FYAGENT_MACOS_SDK_ARCHIVE}"
readonly FYAGENT_MACOS_SDK_SHA256="6e146275d19f027faa2e8354da5e0267513abf013b8f16ad65a231653a2b1c5d"

readonly FYAGENT_OSXCROSS_REPOSITORY="https://github.com/tpoechtrager/osxcross.git"
readonly FYAGENT_OSXCROSS_COMMIT="27d21e4977c9751d01199c7a226a6faf494c3dd9"
readonly FYAGENT_OSXCROSS_BUILD_FLAVOR="llvm"
readonly FYAGENT_MACOSX_DEPLOYMENT_TARGET="12.0"

readonly FYAGENT_LIBDMG_REPOSITORY="https://github.com/planetbeing/libdmg-hfsplus.git"
readonly FYAGENT_LIBDMG_COMMIT="7ac55ec64c96f7800d9818ce64c79670e7f02b67"
readonly FYAGENT_LIBDMG_FILEVAULT_MODE="disabled"

readonly FYAGENT_RCODESIGN_VERSION="0.29.0"
readonly FYAGENT_RCODESIGN_ARCHIVE="apple-codesign-${FYAGENT_RCODESIGN_VERSION}-x86_64-unknown-linux-musl.tar.gz"
readonly FYAGENT_RCODESIGN_URL="https://github.com/indygreg/apple-platform-rs/releases/download/apple-codesign/${FYAGENT_RCODESIGN_VERSION}/${FYAGENT_RCODESIGN_ARCHIVE}"
readonly FYAGENT_RCODESIGN_SHA256="dbe85cedd8ee4217b64e9a0e4c2aef92ab8bcaaa41f20bde99781ff02e600002"

readonly FYAGENT_NODE_VERSION="22.12.0"
readonly FYAGENT_PNPM_VERSION="10.12.3"
readonly FYAGENT_PYTHON_VERSION="3.12.8"
readonly FYAGENT_RUST_VERSION="1.95.0"

FYAGENT_HOST_OS_ID="$(awk -F= '$1 == "ID" { gsub(/"/, "", $2); print $2 }' /etc/os-release)"
FYAGENT_HOST_OS_VERSION_ID="$(awk -F= '$1 == "VERSION_ID" { gsub(/"/, "", $2); print $2 }' /etc/os-release)"
FYAGENT_HOST_ARCH="$(uname -m)"
readonly FYAGENT_HOST_OS_ID FYAGENT_HOST_OS_VERSION_ID FYAGENT_HOST_ARCH
readonly FYAGENT_HOST_CACHE_KEY="${FYAGENT_HOST_OS_ID}-${FYAGENT_HOST_OS_VERSION_ID}-${FYAGENT_HOST_ARCH}"

readonly FYAGENT_MACOS_CROSS_CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/fyagent-macos-cross"
readonly FYAGENT_MACOS_CROSS_DATA_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/fyagent-macos-cross"
readonly FYAGENT_MACOS_CROSS_STATE_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/fyagent-macos-cross"

readonly FYAGENT_OSXCROSS_KEY="${FYAGENT_OSXCROSS_COMMIT}-sdk-${FYAGENT_MACOS_SDK_SHA256}-min-${FYAGENT_MACOSX_DEPLOYMENT_TARGET}-${FYAGENT_HOST_CACHE_KEY}"
readonly FYAGENT_OSXCROSS_TARGET_DIR="${FYAGENT_MACOS_CROSS_DATA_ROOT}/osxcross/${FYAGENT_OSXCROSS_KEY}"
readonly FYAGENT_LIBDMG_INSTALL_DIR="${FYAGENT_MACOS_CROSS_DATA_ROOT}/libdmg/${FYAGENT_LIBDMG_COMMIT}-${FYAGENT_LIBDMG_FILEVAULT_MODE}-${FYAGENT_HOST_CACHE_KEY}"
readonly FYAGENT_RCODESIGN_INSTALL_DIR="${FYAGENT_MACOS_CROSS_DATA_ROOT}/apple-codesign/${FYAGENT_RCODESIGN_VERSION}-${FYAGENT_HOST_CACHE_KEY}"
readonly FYAGENT_LLVM_HOST_WRAPPER_DIR="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/host-wrappers/llvm"
