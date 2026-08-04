#!/usr/bin/env bash
set -euo pipefail
# shellcheck source-path=SCRIPTDIR

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck source=constants.sh
source "$SCRIPT_DIR/constants.sh"
# shellcheck source=lib.sh
source "$SCRIPT_DIR/lib.sh"

install_mise_tools() {
  local actual_path
  [[ -n "${FYAGENT_MISE_BIN:-}" && -x "$FYAGENT_MISE_BIN" ]] || \
    fyagent_die "global mise is missing; install mise and rerun"
  "$FYAGENT_MISE_BIN" trust --yes "$PROJECT_ROOT/mise.toml" >/dev/null
  "$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" install

  local actual_node
  local actual_pnpm
  local actual_python
  local actual_rust
  actual_node="$("$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- node --version)"
  actual_pnpm="$("$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- pnpm --version)"
  actual_python="$("$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- python --version 2>&1)"
  actual_rust="$("$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- rustc --version)"

  [[ "$actual_node" == "v${FYAGENT_NODE_VERSION}" ]] || fyagent_die "mise Node mismatch: $actual_node"
  [[ "$actual_pnpm" == "$FYAGENT_PNPM_VERSION" ]] || fyagent_die "mise pnpm mismatch: $actual_pnpm"
  [[ "$actual_python" == "Python ${FYAGENT_PYTHON_VERSION}" ]] || fyagent_die "mise Python mismatch: $actual_python"
  [[ "$actual_rust" == "rustc ${FYAGENT_RUST_VERSION} "* ]] || fyagent_die "mise Rust mismatch: $actual_rust"

  for actual_path in node pnpm python rustc cargo rustfmt clippy-driver; do
    actual_path="$($FYAGENT_MISE_BIN --cd "$PROJECT_ROOT" which "$actual_path")"
    case "$actual_path" in
      /mnt/*) fyagent_die "global mise resolved a project tool under /mnt: $actual_path" ;;
    esac
  done

  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    "$FYAGENT_MISE_BIN" --cd "$PROJECT_ROOT" exec -- rustup target list --installed | grep -qx "$target" || \
      fyagent_die "mise Rust target is missing: $target"
  done
  fyagent_log "global mise runtimes and Rust targets match the project contract"
}

prepare_llvm_host_wrappers() {
  local linker_candidate
  local otool_candidate

  linker_candidate="$(command -v ld64.lld 2>/dev/null || true)"
  if [[ -z "$linker_candidate" ]]; then
    linker_candidate="$(find /usr/bin -maxdepth 1 \( -type f -o -type l \) \
      -name 'ld64.lld-*' -print | sort -V | tail -n 1)"
  fi
  [[ -n "$linker_candidate" && -x "$linker_candidate" ]] || \
    fyagent_die "Ubuntu LLVM does not provide ld64.lld or a versioned ld64.lld-* executable"

  otool_candidate="$(command -v llvm-otool 2>/dev/null || true)"
  if [[ -z "$otool_candidate" ]]; then
    otool_candidate="$(find /usr/bin -maxdepth 1 \( -type f -o -type l \) \
      -name 'llvm-otool-*' -print | sort -V | tail -n 1)"
  fi
  [[ -n "$otool_candidate" && -x "$otool_candidate" ]] || \
    fyagent_die "Ubuntu LLVM does not provide llvm-otool or a versioned llvm-otool-* executable"

  mkdir -p "$FYAGENT_LLVM_HOST_WRAPPER_DIR"
  ln -sfn -- "$linker_candidate" "$FYAGENT_LLVM_HOST_WRAPPER_DIR/ld64.lld"
  ln -sfn -- "$otool_candidate" "$FYAGENT_LLVM_HOST_WRAPPER_DIR/llvm-otool"
  "$FYAGENT_LLVM_HOST_WRAPPER_DIR/ld64.lld" --version | head -n 1
  "$FYAGENT_LLVM_HOST_WRAPPER_DIR/llvm-otool" --version | head -n 1
  fyagent_log "mapped OSXCross LLVM tools to $linker_candidate and $otool_candidate"
}

install_osxcross() {
  local downloads_root="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/downloads"
  local source_root="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/sources"
  local sdk_path="${downloads_root}/${FYAGENT_MACOS_SDK_ARCHIVE}"
  local source_dir="${source_root}/osxcross-${FYAGENT_OSXCROSS_COMMIT}"
  local marker="${FYAGENT_OSXCROSS_TARGET_DIR}/share/fyagent/toolchain-manifest.txt"
  local temporary_target
  local stale_target
  local actual_commit
  local hello_dir
  local arch

  prepare_llvm_host_wrappers
  if [[ -x "${FYAGENT_OSXCROSS_TARGET_DIR}/bin/osxcross-conf" && -f "$marker" ]] && \
     grep -qx "source_commit=${FYAGENT_OSXCROSS_COMMIT}" "$marker" && \
     grep -qx "sdk_sha256=${FYAGENT_MACOS_SDK_SHA256}" "$marker" && \
     grep -qx "deployment_target=${FYAGENT_MACOSX_DEPLOYMENT_TARGET}" "$marker"; then
    fyagent_log "cache hit: OSXCross $FYAGENT_OSXCROSS_COMMIT"
    return 0
  fi

  fyagent_download_verified "$FYAGENT_MACOS_SDK_URL" "$FYAGENT_MACOS_SDK_SHA256" "$sdk_path"
  fyagent_clone_exact_commit "$FYAGENT_OSXCROSS_REPOSITORY" "$FYAGENT_OSXCROSS_COMMIT" "$source_dir" "$source_root"
  actual_commit="$(git -C "$source_dir" rev-parse HEAD)"
  [[ "$actual_commit" == "$FYAGENT_OSXCROSS_COMMIT" ]] || fyagent_die "OSXCross source commit mismatch"

  mkdir -p "$source_dir/tarballs" "$(dirname "$FYAGENT_OSXCROSS_TARGET_DIR")"
  while IFS= read -r stale_target; do
    fyagent_remove_owned_tree "$stale_target" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
  done < <(find "$(dirname "$FYAGENT_OSXCROSS_TARGET_DIR")" -maxdepth 1 -type d -name '.target.*' -print)
  cp -f -- "$sdk_path" "$source_dir/tarballs/$FYAGENT_MACOS_SDK_ARCHIVE"
  temporary_target="$(mktemp -d "$(dirname "$FYAGENT_OSXCROSS_TARGET_DIR")/.target.XXXXXX")"
  if ! (
    cd "$source_dir"
    export PATH="$FYAGENT_LLVM_HOST_WRAPPER_DIR:$PATH"
    UNATTENDED=1 \
      BUILD_FLAVOR="$FYAGENT_OSXCROSS_BUILD_FLAVOR" \
      TARGET_DIR="$temporary_target" \
      OSX_VERSION_MIN="$FYAGENT_MACOSX_DEPLOYMENT_TARGET" \
      ENABLE_ARCHS="arm64 x86_64" \
      ./build.sh
  ); then
    fyagent_remove_owned_tree "$temporary_target" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
    fyagent_die "OSXCross build failed"
    return 1
  fi

  [[ -x "$temporary_target/bin/osxcross-conf" ]] || fyagent_die "OSXCross did not install osxcross-conf"
  hello_dir="$(mktemp -d "${FYAGENT_MACOS_CROSS_CACHE_ROOT}/.hello.XXXXXX")"
  printf 'int main(void) { return 0; }\n' >"$hello_dir/hello.c"
  for arch in arm64 x86_64; do
    "$temporary_target/bin/xcrun" clang -arch "$arch" "$hello_dir/hello.c" -o "$hello_dir/hello-$arch"
    file "$hello_dir/hello-$arch" | grep -q 'Mach-O 64-bit' || fyagent_die "OSXCross smoke test failed for $arch"
  done
  fyagent_remove_owned_tree "$hello_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"

  mkdir -p "$temporary_target/share/fyagent"
  {
    printf 'installed_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'source_commit=%s\n' "$FYAGENT_OSXCROSS_COMMIT"
    printf 'sdk_url=%s\n' "$FYAGENT_MACOS_SDK_URL"
    printf 'sdk_sha256=%s\n' "$FYAGENT_MACOS_SDK_SHA256"
    printf 'build_flavor=%s\n' "$FYAGENT_OSXCROSS_BUILD_FLAVOR"
    printf 'deployment_target=%s\n' "$FYAGENT_MACOSX_DEPLOYMENT_TARGET"
    printf 'architectures=arm64 x86_64\n'
    printf 'host_ld64_lld=%s\n' "$(readlink -f "$FYAGENT_LLVM_HOST_WRAPPER_DIR/ld64.lld")"
    printf 'host_llvm_otool=%s\n' "$(readlink -f "$FYAGENT_LLVM_HOST_WRAPPER_DIR/llvm-otool")"
  } >"$temporary_target/share/fyagent/toolchain-manifest.txt"

  if [[ -e "$FYAGENT_OSXCROSS_TARGET_DIR" ]]; then
    fyagent_remove_owned_tree "$FYAGENT_OSXCROSS_TARGET_DIR" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
  fi
  mv -- "$temporary_target" "$FYAGENT_OSXCROSS_TARGET_DIR"
  fyagent_log "installed OSXCross $FYAGENT_OSXCROSS_COMMIT"
}

install_libdmg() {
  local source_root="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/sources"
  local source_dir="${source_root}/libdmg-${FYAGENT_LIBDMG_COMMIT}"
  local marker="${FYAGENT_LIBDMG_INSTALL_DIR}/manifest.txt"
  local build_dir
  local temporary_install
  local dmg_binary
  local actual_commit
  local stale_path

  if [[ -x "${FYAGENT_LIBDMG_INSTALL_DIR}/bin/dmg" && -f "$marker" ]] && \
     grep -qx "source_commit=${FYAGENT_LIBDMG_COMMIT}" "$marker" && \
     grep -qx "filevault=${FYAGENT_LIBDMG_FILEVAULT_MODE}" "$marker" && \
     grep -qx "host=${FYAGENT_HOST_CACHE_KEY}" "$marker"; then
    fyagent_log "cache hit: libdmg-hfsplus $FYAGENT_LIBDMG_COMMIT"
    return 0
  fi

  fyagent_clone_exact_commit "$FYAGENT_LIBDMG_REPOSITORY" "$FYAGENT_LIBDMG_COMMIT" "$source_dir" "$source_root"
  actual_commit="$(git -C "$source_dir" rev-parse HEAD)"
  [[ "$actual_commit" == "$FYAGENT_LIBDMG_COMMIT" ]] || fyagent_die "libdmg source commit mismatch"

  mkdir -p "$(dirname "$FYAGENT_LIBDMG_INSTALL_DIR")"
  while IFS= read -r stale_path; do
    fyagent_remove_owned_tree "$stale_path" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"
  done < <(find "$FYAGENT_MACOS_CROSS_CACHE_ROOT" -maxdepth 1 -type d -name '.libdmg-build.*' -print)
  while IFS= read -r stale_path; do
    fyagent_remove_owned_tree "$stale_path" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
  done < <(find "$(dirname "$FYAGENT_LIBDMG_INSTALL_DIR")" -maxdepth 1 -type d -name '.install.*' -print)
  build_dir="$(mktemp -d "${FYAGENT_MACOS_CROSS_CACHE_ROOT}/.libdmg-build.XXXXXX")"
  temporary_install="$(mktemp -d "$(dirname "$FYAGENT_LIBDMG_INSTALL_DIR")/.install.XXXXXX")"
  if ! cmake -S "$source_dir" -B "$build_dir" \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_MODULE_PATH="$SCRIPT_DIR/cmake" || \
     ! cmake --build "$build_dir" --target dmg-bin --parallel; then
    fyagent_remove_owned_tree "$build_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"
    fyagent_remove_owned_tree "$temporary_install" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
    fyagent_die "libdmg build failed"
    return 1
  fi
  dmg_binary="$(find "$build_dir" -type f -name dmg -perm -111 -print -quit)"
  [[ -n "$dmg_binary" ]] || fyagent_die "libdmg build did not produce the dmg executable"
  if ldd "$dmg_binary" | grep -q 'libcrypto'; then
    fyagent_remove_owned_tree "$build_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"
    fyagent_remove_owned_tree "$temporary_install" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
    fyagent_die "libdmg unexpectedly linked libcrypto; FileVault must remain disabled"
    return 1
  fi
  mkdir -p "$temporary_install/bin"
  install -m 0755 "$dmg_binary" "$temporary_install/bin/dmg"
  {
    printf 'installed_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'source_commit=%s\n' "$FYAGENT_LIBDMG_COMMIT"
    printf 'filevault=%s\n' "$FYAGENT_LIBDMG_FILEVAULT_MODE"
    printf 'host=%s\n' "$FYAGENT_HOST_CACHE_KEY"
    printf 'license=GPL-3.0\n'
    printf 'upstream_status=highly-experimental\n'
  } >"$temporary_install/manifest.txt"
  fyagent_remove_owned_tree "$build_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"

  if [[ -e "$FYAGENT_LIBDMG_INSTALL_DIR" ]]; then
    fyagent_remove_owned_tree "$FYAGENT_LIBDMG_INSTALL_DIR" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
  fi
  mv -- "$temporary_install" "$FYAGENT_LIBDMG_INSTALL_DIR"
  fyagent_log "installed libdmg-hfsplus $FYAGENT_LIBDMG_COMMIT"
}

install_rcodesign() {
  local archive="${FYAGENT_MACOS_CROSS_CACHE_ROOT}/downloads/${FYAGENT_RCODESIGN_ARCHIVE}"
  local marker="${FYAGENT_RCODESIGN_INSTALL_DIR}/manifest.txt"
  local extract_dir
  local temporary_install
  local rcodesign_binary
  local version_output

  if [[ -x "${FYAGENT_RCODESIGN_INSTALL_DIR}/bin/rcodesign" && -f "$marker" ]] && \
     grep -qx "version=${FYAGENT_RCODESIGN_VERSION}" "$marker" && \
     grep -qx "archive_sha256=${FYAGENT_RCODESIGN_SHA256}" "$marker" && \
     grep -qx "host=${FYAGENT_HOST_CACHE_KEY}" "$marker"; then
    fyagent_log "cache hit: rcodesign $FYAGENT_RCODESIGN_VERSION"
    return 0
  fi

  mkdir -p "$(dirname "$FYAGENT_RCODESIGN_INSTALL_DIR")"
  fyagent_download_verified "$FYAGENT_RCODESIGN_URL" "$FYAGENT_RCODESIGN_SHA256" "$archive"
  extract_dir="$(mktemp -d "${FYAGENT_MACOS_CROSS_CACHE_ROOT}/.rcodesign-extract.XXXXXX")"
  temporary_install="$(mktemp -d "$(dirname "$FYAGENT_RCODESIGN_INSTALL_DIR")/.install.XXXXXX")"
  tar -xzf "$archive" -C "$extract_dir"
  rcodesign_binary="$(find "$extract_dir" -type f -name rcodesign -perm -111 -print -quit)"
  [[ -n "$rcodesign_binary" ]] || fyagent_die "rcodesign archive does not contain an executable named rcodesign"
  mkdir -p "$temporary_install/bin"
  install -m 0755 "$rcodesign_binary" "$temporary_install/bin/rcodesign"
  version_output="$(fyagent_version_output "$temporary_install/bin/rcodesign" --version)"
  [[ "$version_output" == *"${FYAGENT_RCODESIGN_VERSION}"* ]] || fyagent_die "unexpected rcodesign version: $version_output"
  {
    printf 'installed_at_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'version=%s\n' "$FYAGENT_RCODESIGN_VERSION"
    printf 'archive_url=%s\n' "$FYAGENT_RCODESIGN_URL"
    printf 'archive_sha256=%s\n' "$FYAGENT_RCODESIGN_SHA256"
    printf 'host=%s\n' "$FYAGENT_HOST_CACHE_KEY"
  } >"$temporary_install/manifest.txt"
  fyagent_remove_owned_tree "$extract_dir" "$FYAGENT_MACOS_CROSS_CACHE_ROOT"

  if [[ -e "$FYAGENT_RCODESIGN_INSTALL_DIR" ]]; then
    fyagent_remove_owned_tree "$FYAGENT_RCODESIGN_INSTALL_DIR" "$FYAGENT_MACOS_CROSS_DATA_ROOT"
  fi
  mv -- "$temporary_install" "$FYAGENT_RCODESIGN_INSTALL_DIR"
  fyagent_log "installed rcodesign $FYAGENT_RCODESIGN_VERSION"
}

mkdir -p "$FYAGENT_MACOS_CROSS_CACHE_ROOT" "$FYAGENT_MACOS_CROSS_DATA_ROOT" "$FYAGENT_MACOS_CROSS_STATE_ROOT"
install_mise_tools
install_osxcross
install_libdmg
install_rcodesign
