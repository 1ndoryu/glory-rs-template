#!/usr/bin/env bash
# [028A-9] Bash/Git Bash companion to global-cargo-guard.ps1.
# The PowerShell profile cannot intercept commands launched by Bash. This
# file is sourced by .bashrc/.bash_profile and BASH_ENV so interactive and
# non-interactive agent shells use the same project-aware command policy.

# BASH_ENV sources this file in every non-interactive child shell. Redefining
# the functions is intentional: exported shell functions are not guaranteed
# to survive across Bash versions, while the project root must be resolved
# again after a `cd` or a branch/workspace change.
export GLORY_QUALITY_BASH_GUARD_LOADED=1

GLORY_QUALITY_BASH_GUARD_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
export GLORY_QUALITY_BASH_GUARD_DIR
case ":${PATH:-}:" in
  *":${GLORY_QUALITY_BASH_GUARD_DIR}:"*) ;;
  *) export PATH="${GLORY_QUALITY_BASH_GUARD_DIR}:${PATH:-}" ;;
esac

glory_quality_find_root() {
  local candidate="${PWD:-.}"
  while [[ -n "$candidate" && "$candidate" != "/" ]]; do
    if [[ -f "$candidate/quality.config.json" && -f "$candidate/scripts/quality/quality-command-guard.mjs" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
    local parent
    parent="$(dirname -- "$candidate")"
    [[ "$parent" == "$candidate" ]] && break
    candidate="$parent"
  done
  return 1
}

glory_quality_host_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s\n' "$1"
  fi
}

glory_quality_guard_command() {
  local executable="$1"
  shift
  local root
  root="$(glory_quality_find_root 2>/dev/null)" || return 0
  local node_bin="${GLORY_REAL_NODE:-node}"
  local root_host
  root_host="$(glory_quality_host_path "$root")"
  local guard_script
  guard_script="$(glory_quality_host_path "$root/scripts/quality/quality-command-guard.mjs")"
  "$node_bin" "$guard_script" --project-root "$root_host" --executable "$executable" -- "$@"
}

glory_quality_real_command() {
  local name="$1"
  local configured=""
  case "$name" in
    cargo) configured="${GLORY_REAL_CARGO:-}" ;;
    npm) configured="${GLORY_REAL_NPM:-}" ;;
    npx) configured="${GLORY_REAL_NPX:-}" ;;
  esac
  if [[ -n "$configured" ]]; then
    if command -v cygpath >/dev/null 2>&1; then
      configured="$(cygpath -u "$configured" 2>/dev/null || printf '%s' "$configured")"
    fi
    printf '%s\n' "$configured"
    return 0
  fi

  local candidate
  candidate="$(type -P "${name}.exe" 2>/dev/null || true)"
  [[ -n "$candidate" ]] && { printf '%s\n' "$candidate"; return 0; }
  candidate="$(type -P "$name" 2>/dev/null || true)"
  if [[ -n "$candidate" && "$candidate" != "$GLORY_QUALITY_BASH_GUARD_DIR/$name" ]]; then
    printf '%s\n' "$candidate"
    return 0
  fi
  return 1
}

glory_quality_dispatch() {
  local name="$1"
  shift
  glory_quality_guard_command "$name" "$@"
  local guard_exit=$?
  [[ $guard_exit -eq 0 ]] || return "$guard_exit"

  local real_command
  real_command="$(glory_quality_real_command "$name" 2>/dev/null)" || {
    printf '[glory-quality] No se encontró el ejecutable real de %s.\n' "$name" >&2
    return 127
  }
  "$real_command" "$@"
}

cargo() { glory_quality_dispatch cargo "$@"; }
rustfmt() { glory_quality_dispatch rustfmt "$@"; }
npm() { glory_quality_dispatch npm "$@"; }
npx() { glory_quality_dispatch npx "$@"; }
vitest() { glory_quality_dispatch vitest "$@"; }
tsc() { glory_quality_dispatch tsc "$@"; }
eslint() { glory_quality_dispatch eslint "$@"; }
prettier() { glory_quality_dispatch prettier "$@"; }

# Child Bash processes source this file even when they are non-interactive.
export BASH_ENV="${BASH_ENV:-${GLORY_QUALITY_BASH_GUARD_DIR}/global-quality-guard.sh}"
