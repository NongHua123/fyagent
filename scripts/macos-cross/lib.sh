#!/usr/bin/env bash

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "error: source this file instead of executing it" >&2
  exit 2
fi

fyagent_log() {
  printf '[macos-cross] %s\n' "$*"
}

fyagent_die() {
  printf '[macos-cross] error: %s\n' "$*" >&2
  return 1
}

fyagent_require_command() {
  local command_name="$1"
  command -v "$command_name" >/dev/null 2>&1 || fyagent_die "required command is missing: $command_name"
}

fyagent_sha256() {
  sha256sum "$1" | awk '{print $1}'
}

fyagent_download_verified() {
  local url="$1"
  local expected_sha256="$2"
  local destination="$3"
  local destination_dir
  local actual_sha256
  local partial_file

  destination_dir="$(dirname "$destination")"
  mkdir -p "$destination_dir"

  if [[ -f "$destination" ]]; then
    actual_sha256="$(fyagent_sha256 "$destination")"
    if [[ "$actual_sha256" == "$expected_sha256" ]]; then
      fyagent_log "cache hit: $(basename "$destination")"
      return 0
    fi
    fyagent_log "discarding cached file with an invalid checksum: $destination"
    rm -f -- "$destination"
  fi

  partial_file="${destination}.part"
  if ! curl --fail --location --proto '=https' --tlsv1.2 \
    --retry 4 --retry-all-errors --connect-timeout 30 \
    --continue-at - --progress-bar --output "$partial_file" "$url"; then
    fyagent_die "download failed: $url (partial data retained for the next retry)"
    return 1
  fi

  actual_sha256="$(fyagent_sha256 "$partial_file")"
  if [[ "$actual_sha256" != "$expected_sha256" ]]; then
    rm -f -- "$partial_file"
    fyagent_die "checksum mismatch for $url (expected $expected_sha256, got $actual_sha256)"
    return 1
  fi

  mv -f -- "$partial_file" "$destination"
  fyagent_log "downloaded and verified: $(basename "$destination")"
}

fyagent_assert_owned_path() {
  local candidate="$1"
  local allowed_root="$2"
  local normalized_candidate
  local normalized_root

  normalized_candidate="$(realpath -m -- "$candidate")"
  normalized_root="$(realpath -m -- "$allowed_root")"
  case "$normalized_candidate" in
    "$normalized_root"/*) ;;
    *) fyagent_die "refusing to mutate path outside $normalized_root: $normalized_candidate" ;;
  esac
}

fyagent_remove_owned_tree() {
  local candidate="$1"
  local allowed_root="$2"
  fyagent_assert_owned_path "$candidate" "$allowed_root"
  rm -rf -- "$candidate"
}

fyagent_clone_exact_commit() {
  local repository="$1"
  local commit="$2"
  local destination="$3"
  local source_root="$4"
  local actual_commit
  local temporary_dir

  if [[ -d "$destination/.git" ]]; then
    actual_commit="$(git -C "$destination" rev-parse HEAD 2>/dev/null || true)"
    if [[ "$actual_commit" == "$commit" ]]; then
      fyagent_log "cache hit: $(basename "$destination") at $commit"
      return 0
    fi
    fyagent_remove_owned_tree "$destination" "$source_root"
  elif [[ -e "$destination" ]]; then
    fyagent_remove_owned_tree "$destination" "$source_root"
  fi

  mkdir -p "$source_root"
  temporary_dir="$(mktemp -d "${source_root}/.clone.XXXXXX")"
  if ! git clone --filter=blob:none --no-checkout "$repository" "$temporary_dir"; then
    fyagent_remove_owned_tree "$temporary_dir" "$source_root"
    fyagent_die "git clone failed: $repository"
    return 1
  fi
  if ! git -C "$temporary_dir" fetch --depth 1 origin "$commit" || \
     ! git -C "$temporary_dir" checkout --detach "$commit"; then
    fyagent_remove_owned_tree "$temporary_dir" "$source_root"
    fyagent_die "cannot resolve pinned commit $commit from $repository"
    return 1
  fi

  actual_commit="$(git -C "$temporary_dir" rev-parse HEAD)"
  if [[ "$actual_commit" != "$commit" ]]; then
    fyagent_remove_owned_tree "$temporary_dir" "$source_root"
    fyagent_die "checked-out commit mismatch for $repository"
    return 1
  fi
  mv -- "$temporary_dir" "$destination"
}

fyagent_version_output() {
  local executable="$1"
  shift
  "$executable" "$@" 2>&1 | head -n 1
}
