#!/usr/bin/env sh
# install.sh — one-command Astrid installer (base OS + optional oracles)
#
# Website / GitHub Pages / raw:
#   curl -fsSL https://astridos.org/install.sh | sh
#
# Product model:
#   * Base Astrid is a complete install (daemon, CLI, default principal).
#   * Oracles (Claude / Grok / Codex) are optional host adapters on top.
#
# Behaviour (least surprise):
#   1. Ensure the Astrid CLI is available (Homebrew / PATH / dev tree)
#   2. Ensure base runtime is initialized (astrid init -y when needed)
#   3. Detect Claude Code / Grok / Codex on this machine
#   4. Wire only those hosts (shared astrid-mcp + host distro) — never force all
#   5. If none detected → stop at base Astrid (success, not a half-install)
#
# Flags:
#   --host claude|grok|codex   wire only this host (repeatable)
#   --all                      wire every host (power users / demos)
#   --yes / -y                 non-interactive (default when not a TTY)
#   --no-brew                  never invoke Homebrew
#   --bin-root PATH            use this directory for astrid + astrid-daemon
#   --skip-init                install/detect only; do not run astrid init
#   -h / --help

set -eu

# ---------------------------------------------------------------------------
# Defaults — override with env for mirrors / forks
# ---------------------------------------------------------------------------
ORACLES_REPO="${ASTRID_ORACLES_REPO:-unicity-astrid/oracles}"
ORACLES_REF="${ASTRID_ORACLES_REF:-main}"
DISTRO_BASE="${ASTRID_ORACLES_DISTRO_BASE:-https://raw.githubusercontent.com/${ORACLES_REPO}/${ORACLES_REF}/distros}"
BREW_TAP="${ASTRID_BREW_TAP:-unicity-astrid/tap}"
BREW_FORMULA="${ASTRID_BREW_FORMULA:-astrid}"

# ---------------------------------------------------------------------------
# UI
# ---------------------------------------------------------------------------
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  C_BOLD='\033[1m'
  C_DIM='\033[2m'
  C_GREEN='\033[32m'
  C_YELLOW='\033[33m'
  C_CYAN='\033[36m'
  C_RED='\033[31m'
  C_RESET='\033[0m'
else
  C_BOLD='' C_DIM='' C_GREEN='' C_YELLOW='' C_CYAN='' C_RED='' C_RESET=''
fi

say()  { printf '%b\n' "$*"; }
info() { say "${C_CYAN}→${C_RESET} $*"; }
ok()   { say "${C_GREEN}✓${C_RESET} $*"; }
warn() { say "${C_YELLOW}!${C_RESET} $*"; }
err()  { say "${C_RED}✗${C_RESET} $*" >&2; }
die()  { err "$*"; exit 1; }
header() {
  say ""
  say "${C_BOLD}$*${C_RESET}"
  say "${C_DIM}$(printf '%*s' "${#1}" '' | tr ' ' '─')${C_RESET}"
}

usage() {
  cat <<'EOF'
Astrid Oracles installer

  curl -fsSL https://astridos.org/install.sh | sh

Options:
  --host NAME     Wire claude | grok | codex (repeatable)
  --all           Wire every host (not the default)
  --yes, -y       Non-interactive (auto-yes; default when stdin is not a TTY)
  --no-brew       Do not use Homebrew if astrid is missing
  --bin-root DIR  Prefer astrid + astrid-daemon from DIR
  --skip-init     Skip astrid init / principal provisioning
  -h, --help      Show this help

Env:
  ASTRID_ORACLES_REPO          default unicity-astrid/oracles
  ASTRID_ORACLES_REF           default main
  ASTRID_ORACLES_DISTRO_BASE   raw base URL for distro tomls
  ASTRID_BIN_ROOT              same as --bin-root
EOF
}

# ---------------------------------------------------------------------------
# Args
# ---------------------------------------------------------------------------
YES=0
NO_BREW=0
SKIP_INIT=0
ALL_HOSTS=0
BASE_ONLY=0
BIN_ROOT="${ASTRID_BIN_ROOT:-}"
# space-separated list of hosts requested via --host
REQUESTED_HOSTS=""

# Non-interactive when not a TTY (curl | sh)
if [ ! -t 0 ]; then
  YES=1
fi

while [ "$#" -gt 0 ]; do
  case "$1" in
    --host)
      shift
      h="${1:-}"
      case "$h" in
        claude|grok|codex) REQUESTED_HOSTS="${REQUESTED_HOSTS} ${h}" ;;
        *) die "unknown host '$h' (want claude|grok|codex)" ;;
      esac
      ;;
    --all) ALL_HOSTS=1 ;;
    --base-only) BASE_ONLY=1 ;;
    --yes|-y) YES=1 ;;
    --no-brew) NO_BREW=1 ;;
    --bin-root)
      shift
      BIN_ROOT="${1:-}"
      [ -n "$BIN_ROOT" ] || die "--bin-root needs a path"
      ;;
    --skip-init) SKIP_INIT=1 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done

# ---------------------------------------------------------------------------
# Detect hosts already on the machine
# ---------------------------------------------------------------------------
have_cmd() { command -v "$1" >/dev/null 2>&1; }

detect_claude() {
  have_cmd claude && return 0
  [ -x "${HOME}/.claude/local/claude" ] && return 0
  [ -d "${HOME}/.claude" ] && return 0
  return 1
}

detect_grok() {
  have_cmd grok && return 0
  [ -d "${HOME}/.grok" ] && return 0
  return 1
}

detect_codex() {
  have_cmd codex && return 0
  [ -d "${HOME}/.codex" ] && return 0
  return 1
}

# ---------------------------------------------------------------------------
# Astrid binary resolution
# ---------------------------------------------------------------------------
has_pair() {
  root="$1"
  [ -n "$root" ] && [ -x "$root/astrid" ] && [ -x "$root/astrid-daemon" ]
}

resolve_astrid() {
  if [ -n "$BIN_ROOT" ]; then
    has_pair "$BIN_ROOT" || die "--bin-root missing astrid + astrid-daemon: $BIN_ROOT"
    ASTRID="$BIN_ROOT/astrid"
    export ASTRID_BIN_ROOT="$BIN_ROOT"
    ok "using Astrid from $BIN_ROOT"
    return 0
  fi

  if [ -n "${ASTRID_BIN:-}" ] && [ -x "$ASTRID_BIN" ]; then
    ASTRID="$ASTRID_BIN"
    ok "using ASTRID_BIN=$ASTRID"
    return 0
  fi

  if have_cmd astrid; then
    ASTRID="$(command -v astrid)"
    ok "found astrid on PATH: $ASTRID"
    return 0
  fi

  # Dev trees near cwd
  dir="$(pwd -P 2>/dev/null || pwd)"
  while [ "$dir" != "/" ] && [ -n "$dir" ]; do
    for sub in core/target/debug core/target/release target/debug target/release; do
      if has_pair "$dir/$sub"; then
        ASTRID="$dir/$sub/astrid"
        export ASTRID_BIN_ROOT="$dir/$sub"
        ok "found dev Astrid at $ASTRID_BIN_ROOT"
        return 0
      fi
    done
    dir="$(dirname "$dir")"
  done

  if [ "$NO_BREW" -eq 1 ]; then
    die "astrid not found (and --no-brew). Install: brew install ${BREW_TAP}/${BREW_FORMULA}"
  fi

  if ! have_cmd brew; then
    die "astrid not found and Homebrew missing.
Install Homebrew from https://brew.sh then re-run, or:
  brew install ${BREW_TAP}/${BREW_FORMULA}"
  fi

  info "installing Astrid via Homebrew…"
  brew tap "$BREW_TAP" 2>/dev/null || true
  brew install "${BREW_TAP}/${BREW_FORMULA}" || brew install "$BREW_FORMULA" \
    || die "Homebrew install failed"
  ASTRID="$(command -v astrid)" || die "astrid still not on PATH after brew install"
  ok "installed $ASTRID"
}

astrid_version() {
  "$ASTRID" --version 2>/dev/null | head -n1 || printf 'unknown'
}


# ---------------------------------------------------------------------------
# Base Astrid (complete product without any oracle host)
# ---------------------------------------------------------------------------
base_already_initialized() {
  # Heuristic: default principal home or local capsules dir exists.
  home="${ASTRID_HOME:-$HOME/.astrid}"
  [ -d "$home/home/default" ] || [ -d "$home/home" ] || [ -f "$home/config.toml" ]
}

ensure_base_astrid() {
  header "Base Astrid"
  if [ "$SKIP_INIT" -eq 1 ]; then
    warn "skipping base init (--skip-init)"
    return 0
  fi
  if base_already_initialized; then
    ok "runtime home present (${ASTRID_HOME:-$HOME/.astrid})"
    return 0
  fi
  info "initializing default principal (base install)…"
  if "$ASTRID" init -y 2>&1; then
    ok "base Astrid initialized"
  else
    warn "astrid init -y reported an error — run: astrid doctor"
  fi
}

# ---------------------------------------------------------------------------
# Oracle hosts (optional)
# ---------------------------------------------------------------------------
# Prefer local checkout distros when this script lives in a clone.
script_dir() {
  # When piped to sh, $0 is often "sh" — no local tree.
  case "$0" in
    /*|//*) dirname "$0" ;;
    ./*|../*|*/*) CDPATH= cd -- "$(dirname "$0")" 2>/dev/null && pwd -P ;;
    *) printf '' ;;
  esac
}

distro_url() {
  host="$1"
  local_root="$(script_dir)"
  if [ -n "$local_root" ] && [ -f "$local_root/distros/${host}.toml" ]; then
    printf '%s\n' "$local_root/distros/${host}.toml"
    return 0
  fi
  printf '%s/%s.toml\n' "$DISTRO_BASE" "$host"
}

principal_for() {
  case "$1" in
    claude) printf 'claude-code\n' ;;
    grok)   printf 'grok-code\n' ;;
    codex)  printf 'codex-code\n' ;;
    *)      die "principal_for: bad host $1" ;;
  esac
}

pretty_host() {
  case "$1" in
    claude) printf 'Claude Code\n' ;;
    grok)   printf 'Grok Build\n' ;;
    codex)  printf 'Codex\n' ;;
  esac
}

wire_host() {
  host="$1"
  principal="$(principal_for "$host")"
  distro="$(distro_url "$host")"
  pretty="$(pretty_host "$host")"

  header "Wire $pretty → principal $principal"

  if [ "$SKIP_INIT" -eq 1 ]; then
    warn "skipping astrid init (--skip-init)"
    return 0
  fi

  info "distro: $distro"

  # default first (daemon uplink lives there), then per-principal
  info "provisioning default principal (daemon uplink)…"
  if ! "$ASTRID" init --distro "$distro" -y 2>&1; then
    warn "init for default failed — continuing with principal $principal"
  fi

  info "provisioning $principal…"
  "$ASTRID" init --distro "$distro" --principal "$principal" -y \
    || die "astrid init failed for $principal"

  # Best-effort pre-grant of common capsules (matches doctor guidance)
  case "$host" in
    claude)
      grants="claude-runner astrid-mcp claude-install astrid-capsule-cli astrid-capsule-forge astrid-capsule-fs astrid-capsule-http astrid-capsule-shell astrid-capsule-skills astrid-capsule-system"
      ;;
    codex)
      grants="codex-runner astrid-mcp codex-install astrid-capsule-cli astrid-capsule-forge astrid-capsule-fs astrid-capsule-http astrid-capsule-shell astrid-capsule-skills astrid-capsule-system"
      ;;
    grok)
      grants="astrid-mcp astrid-capsule-cli astrid-capsule-forge astrid-capsule-fs astrid-capsule-http astrid-capsule-shell astrid-capsule-skills astrid-capsule-system"
      ;;
  esac
  add_args=""
  for c in $grants; do
    add_args="$add_args --add-capsule $c"
  done
  # shellcheck disable=SC2086
  if "$ASTRID" agent modify "$principal" $add_args 2>/dev/null; then
    ok "granted distro capsules to $principal"
  else
    warn "could not pre-grant capsules (ok if astrid agent modify is unavailable)"
  fi

  ok "$pretty ready under principal ${C_BOLD}$principal${C_RESET}"
}

plugin_hint() {
  host="$1"
  case "$host" in
    claude)
      say "  ${C_DIM}Plugin:${C_RESET} claude plugin marketplace add ${ORACLES_REPO}"
      say "           then enable the ${C_BOLD}astrid${C_RESET} plugin (Claude Code)"
      ;;
    grok)
      say "  ${C_DIM}Plugin:${C_RESET} grok plugin install <path-to-oracles>/plugins/grok --trust"
      say "           (or clone ${ORACLES_REPO} and point Grok at plugins/grok)"
      ;;
    codex)
      say "  ${C_DIM}Plugin:${C_RESET} install plugins/codex from ${ORACLES_REPO}"
      say "           (Codex marketplace: .agents/plugins)"
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  say ""
  say "${C_BOLD}Astrid${C_RESET}"
  say "${C_DIM}secure OS for AI agents · optional oracles for Claude · Grok · Codex${C_RESET}"
  say ""

  resolve_astrid
  info "version: $(astrid_version)"
  ensure_base_astrid

  # Build host list
  hosts=""
  if [ "$ALL_HOSTS" -eq 1 ]; then
    hosts="claude grok codex"
    info "mode: --all (every host)"
  elif [ -n "$REQUESTED_HOSTS" ]; then
    hosts="$REQUESTED_HOSTS"
    info "mode: explicit --host"
  else
    detected=""
    detect_claude && detected="${detected} claude"
    detect_grok   && detected="${detected} grok"
    detect_codex  && detected="${detected} codex"
    hosts="$detected"
    if [ -n "$hosts" ]; then
      info "detected oracles:$(printf '%s' "$hosts" | sed 's/ /, /g')"
    else
      info "no coding-host oracle detected (base Astrid is enough to start)"
    fi
  fi

  if [ "$BASE_ONLY" -eq 1 ]; then
    hosts=""
    info "mode: --base-only (skip oracle hosts)"
  fi

  # Interactive confirm when multiple detected and TTY
  if [ "$YES" -eq 0 ] && [ -t 0 ] && [ -n "$hosts" ] && [ "$ALL_HOSTS" -eq 0 ] && [ -z "$REQUESTED_HOSTS" ]; then
    count=0
    for _ in $hosts; do count=$((count + 1)); done
    if [ "$count" -gt 1 ]; then
      say ""
      printf "Wire all detected hosts? [Y/n] "
      read -r ans || ans=Y
      case "$ans" in
        n|N|no|No) 
          say "Pick hosts with: install.sh --host claude"
          exit 0
          ;;
      esac
    fi
  fi

  if [ -z "$hosts" ]; then
    header "Base Astrid ready"
    ok "You have a full Astrid install — no coding-host oracle required"
    say ""
    say "Try:"
    say "  ${C_BOLD}astrid doctor${C_RESET}     health check"
    say "  ${C_BOLD}astrid chat${C_RESET}       session (if your distro includes a model)"
    say ""
    say "Optional — wire a coding host when you use one:"
    say "  curl -fsSL https://astridos.org/install.sh | sh -s -- --host claude"
    say "  ${C_DIM}(or re-run this installer after installing Claude / Grok / Codex)${C_RESET}"
    say ""
    exit 0
  fi

  for h in $hosts; do
    wire_host "$h"
  done

  header "Done"
  ok "Base Astrid + oracle host(s)"
  say "Shared tool backend: ${C_BOLD}astrid-mcp${C_RESET}"
  say "Oracles wired:"
  for h in $hosts; do
    say "  • $(pretty_host "$h")  →  $(principal_for "$h")"
    plugin_hint "$h"
  done
  say ""
  say "Verify:  ${C_BOLD}astrid doctor${C_RESET}"
  say "Docs:    https://github.com/${ORACLES_REPO}"
  say ""
}

main
