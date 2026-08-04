#!/usr/bin/env bash
# This file is sourced inside the mise-managed build process.
# shellcheck source-path=SCRIPTDIR

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "error: source this file instead of executing it" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"

[[ -x "${FYAGENT_OSXCROSS_TARGET_DIR}/bin/osxcross-conf" ]] || {
  echo "error: OSXCross is not provisioned at $FYAGENT_OSXCROSS_TARGET_DIR" >&2
  return 1
}

export PATH="${FYAGENT_RCODESIGN_INSTALL_DIR}/bin:${FYAGENT_LLVM_HOST_WRAPPER_DIR}:${FYAGENT_OSXCROSS_TARGET_DIR}/bin:$PATH"
# osxcross-conf intentionally emits shell exports.
eval "$("${FYAGENT_OSXCROSS_TARGET_DIR}/bin/osxcross-conf")"
export SDKROOT="${OSXCROSS_SDK:?osxcross-conf did not export OSXCROSS_SDK}"
export MACOSX_DEPLOYMENT_TARGET="$FYAGENT_MACOSX_DEPLOYMENT_TARGET"

wrapper_root="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/wrappers/${FYAGENT_OSXCROSS_KEY}"
mkdir -p "$wrapper_root"

cat >"$wrapper_root/macos-arm64-clang" <<'WRAPPER'
#!/usr/bin/env bash
exec xcrun clang -arch arm64 "$@"
WRAPPER
cat >"$wrapper_root/macos-arm64-clang++" <<'WRAPPER'
#!/usr/bin/env bash
exec xcrun clang++ -arch arm64 "$@"
WRAPPER
cat >"$wrapper_root/macos-x86_64-clang" <<'WRAPPER'
#!/usr/bin/env bash
exec xcrun clang -arch x86_64 "$@"
WRAPPER
cat >"$wrapper_root/macos-x86_64-clang++" <<'WRAPPER'
#!/usr/bin/env bash
exec xcrun clang++ -arch x86_64 "$@"
WRAPPER
cat >"$wrapper_root/lipo" <<'WRAPPER'
#!/usr/bin/env bash
exec xcrun lipo "$@"
WRAPPER
chmod 0755 "$wrapper_root"/*
export PATH="$wrapper_root:$PATH"

export CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="$wrapper_root/macos-arm64-clang"
export CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER="$wrapper_root/macos-x86_64-clang"

export CC_aarch64_apple_darwin="$wrapper_root/macos-arm64-clang"
export CXX_aarch64_apple_darwin="$wrapper_root/macos-arm64-clang++"
AR_aarch64_apple_darwin="$(xcrun -f ar)"
RANLIB_aarch64_apple_darwin="$(xcrun -f ranlib)"
export AR_aarch64_apple_darwin RANLIB_aarch64_apple_darwin
export CC_x86_64_apple_darwin="$wrapper_root/macos-x86_64-clang"
export CXX_x86_64_apple_darwin="$wrapper_root/macos-x86_64-clang++"
AR_x86_64_apple_darwin="$(xcrun -f ar)"
RANLIB_x86_64_apple_darwin="$(xcrun -f ranlib)"
export AR_x86_64_apple_darwin RANLIB_x86_64_apple_darwin

export CFLAGS_aarch64_apple_darwin="-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export CXXFLAGS_aarch64_apple_darwin="-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export CFLAGS_x86_64_apple_darwin="-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export CXXFLAGS_x86_64_apple_darwin="-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="--sysroot=$SDKROOT -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET -arch arm64"
export BINDGEN_EXTRA_CLANG_ARGS_x86_64_apple_darwin="--sysroot=$SDKROOT -mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET -arch x86_64"

unset CC CXX AR RANLIB
unset PKG_CONFIG_PATH PKG_CONFIG_LIBDIR PKG_CONFIG_SYSROOT_DIR PKG_CONFIG_ALLOW_CROSS
