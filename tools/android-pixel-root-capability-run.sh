#!/usr/bin/env bash
set -euo pipefail

readonly VERSION="v0.1.0"
readonly ROOT_FLAG="--root"
readonly TERMUX_BASH="/data/data/com.termux/files/usr/bin/bash"
readonly SCANNER_BASENAME="pixel_local__usbeehive-android-capability-scan.sh"

print_kv() {
  printf '%s=%s\n' "$1" "$2"
}

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  printf 'RESULT: USBEEHIVE_ANDROID_ROOT_CAPABILITY_SCAN_DONE rc=1\n'
  exit 1
}

main() {
  local script_dir=""
  local self_path=""
  local scanner_path=""
  local rc=0

  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  self_path="$script_dir/$(basename "${BASH_SOURCE[0]}")"
  scanner_path="${USBEEHIVE_SCANNER_PATH:-$script_dir/$SCANNER_BASENAME}"

  printf '== usbeehive Android Pixel root capability runner ==\n'
  print_kv version "$VERSION"
  print_kv generated_at "$(date -Is 2>/dev/null || date)"
  print_kv initial_uid "$(id -u)"
  print_kv scanner_path "$scanner_path"

  [[ -x "$TERMUX_BASH" ]] || fail "termux_bash_missing=$TERMUX_BASH"
  [[ -s "$scanner_path" ]] || fail "scanner_missing_or_empty=$scanner_path"
  "$TERMUX_BASH" -n "$scanner_path" || fail "scanner_bash_syntax"

  if [[ "${1:-}" != "$ROOT_FLAG" ]]; then
    [[ "$(id -u)" != "0" ]] || fail "unexpected_root_without_root_flag"
    command -v su >/dev/null 2>&1 || fail "su_missing"
    print_kv escalation read_only_su_c
    if su -c "$TERMUX_BASH $self_path $ROOT_FLAG"; then
      rc=0
    else
      rc=$?
    fi
    if (( rc != 0 )); then
      printf 'FAIL: root_scan_child_rc=%s\n' "$rc" >&2
      printf 'RESULT: USBEEHIVE_ANDROID_ROOT_CAPABILITY_SCAN_DONE rc=%s\n' "$rc"
      exit "$rc"
    fi
    printf 'RESULT: USBEEHIVE_ANDROID_ROOT_CAPABILITY_SCAN_DONE rc=0\n'
    return 0
  fi

  [[ "$(id -u)" == "0" ]] || fail "root_context_not_acquired"
  print_kv root_uid "$(id -u)"
  print_kv root_user "$(id -un 2>/dev/null || printf root)"
  if command -v getenforce >/dev/null 2>&1; then
    print_kv selinux_mode "$(getenforce 2>/dev/null || printf unknown)"
  fi
  print_kv write_operations none
  print_kv selinux_changes none
  print_kv module_changes none

  "$TERMUX_BASH" "$scanner_path"
  printf 'RESULT: USBEEHIVE_ANDROID_ROOT_CHILD_SCAN_DONE rc=0\n'
}

main "$@"
