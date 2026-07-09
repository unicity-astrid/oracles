#!/usr/bin/env sh
# install.sh — quiet one-command Astrid installer
#
#   curl -fsSL https://astridos.org/install.sh | sh
#
# Ownership (keep this thin):
#   * This script  → CLI + bootable base; announce host plugins
#   * Marketplace  → editor plugin (MCP, doctor, hooks)
#   * astrid init  → capsules (doctor offers on SessionStart; --host does it now)
#   * astrid update → CLI upgrade the Astrid way
#
# Quiet by default. Pass --verbose for chatter.
set -eu

ORACLES_REPO="${ASTRID_ORACLES_REPO:-unicity-astrid/oracles}"
DISTRO_BASE="${ASTRID_ORACLES_DISTRO_BASE:-https://raw.githubusercontent.com/${ORACLES_REPO}/main/distros}"
ASTRID_RELEASE_REPO="${ASTRID_RELEASE_REPO:-unicity-astrid/astrid}"
ASTRID_MANAGED_BIN="${ASTRID_HOME:-$HOME/.astrid}/bin"
BREW_TAP="${ASTRID_BREW_TAP:-unicity-astrid/tap}"
BREW_FORMULA="${ASTRID_BREW_FORMULA:-astrid}"

VERBOSE=0
NO_BREW=0
BASE_ONLY=0
SKIP_INIT=0
ALL_HOSTS=0
REQUESTED_HOSTS=""
BIN_ROOT="${ASTRID_BIN_ROOT:-}"
ASTRID=""

have() { command -v "$1" >/dev/null 2>&1; }
say()  { printf '%s\n' "$*"; }
v()    { [ "$VERBOSE" -eq 1 ] && say "  $*" || true; }
die()  { say "error: $*" >&2; exit 1; }
step() { say "• $*"; }
ok()   { say "  ✓ $*"; }

usage() {
  cat <<'EOF'
Astrid installer (quiet)

  curl -fsSL https://astridos.org/install.sh | sh

Ensures the CLI, a bootable base home, and points you at host plugins.
Capsules are provisioned by the SessionStart doctor (or --host).

  --host NAME   also run distro init for claude|grok|codex (repeatable)
  --all         --host for every host
  --base-only   skip host detection / plugin hints
  --no-brew     never use Homebrew
  --bin-root D  use astrid from D
  --verbose     more output
  -h, --help
EOF
}

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
    --no-brew) NO_BREW=1 ;;
    --bin-root)
      shift
      BIN_ROOT="${1:-}"
      [ -n "$BIN_ROOT" ] || die "--bin-root needs a path"
      ;;
    --skip-init) SKIP_INIT=1 ;;
    --verbose|-v) VERBOSE=1 ;;
    --yes|-y|--upgrade) ;; # accepted, no-op (always non-interactive + upgrade-friendly)
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done

# --- GitHub token (capsule/API paths; also helps astrid update) ------------
if [ -z "${GH_TOKEN:-}${GITHUB_TOKEN:-}" ] && have gh; then
  _tok="$(gh auth token 2>/dev/null || true)"
  [ -n "$_tok" ] && export GH_TOKEN="$_tok" && v "using GH_TOKEN from gh auth"
fi

has_pair() { [ -n "$1" ] && [ -x "$1/astrid" ] && [ -x "$1/astrid-daemon" ]; }

platform_target() {
  os="$(uname -s 2>/dev/null || echo unknown)"
  arch="$(uname -m 2>/dev/null || echo unknown)"
  case "${os}/${arch}" in
    Darwin/arm64|Darwin/aarch64) printf 'aarch64-apple-darwin\n' ;;
    Darwin/x86_64)               printf 'x86_64-apple-darwin\n' ;;
    Linux/x86_64|Linux/amd64)    printf 'x86_64-unknown-linux-gnu\n' ;;
    Linux/aarch64|Linux/arm64)   printf 'aarch64-unknown-linux-gnu\n' ;;
    *) die "unsupported platform ${os}/${arch}" ;;
  esac
}

sha256_file() {
  if have sha256sum; then sha256sum "$1" | awk '{ print $1 }'
  elif have shasum; then shasum -a 256 "$1" | awk '{ print $1 }'
  else die "need sha256sum or shasum"
  fi
}

install_from_github() {
  target="$(platform_target)"
  tmp="$(mktemp -d 2>/dev/null || mktemp -d -t astrid-install)"
  # shellcheck disable=SC2064
  trap 'rm -rf "$tmp" 2>/dev/null || true' EXIT

  if [ -n "${ASTRID_VERSION:-}" ]; then
    tag="v${ASTRID_VERSION#v}"
  else
    have curl || die "curl required to install from GitHub releases"
    api="https://api.github.com/repos/${ASTRID_RELEASE_REPO}/releases/latest"
    meta="$(curl -fsSL --max-time 30 \
      ${GH_TOKEN:+-H "Authorization: Bearer ${GH_TOKEN}"} \
      ${GITHUB_TOKEN:+-H "Authorization: Bearer ${GITHUB_TOKEN}"} \
      "$api")" || die "could not query GitHub releases"
    tag="$(printf '%s' "$meta" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
    [ -n "$tag" ] || die "latest release has no tag_name"
  fi
  version="${tag#v}"
  asset="astrid-${version}-${target}.tar.gz"
  base="https://github.com/${ASTRID_RELEASE_REPO}/releases/download/${tag}"

  v "downloading $asset"
  curl -fsSL --max-time 120 -o "$tmp/$asset" "${base}/${asset}" \
    || die "download failed: ${base}/${asset}"
  curl -fsSL --max-time 30 -o "$tmp/SHA256SUMS.txt" "${base}/SHA256SUMS.txt" \
    || die "could not download SHA256SUMS.txt"
  expected="$(awk -v a="$asset" '$2 == a || $2 == "./"a || index($0, a) { print $1; exit }' "$tmp/SHA256SUMS.txt")"
  [ -n "$expected" ] || die "no checksum for $asset"
  actual="$(sha256_file "$tmp/$asset")"
  [ "$expected" = "$actual" ] || die "checksum mismatch for $asset"

  mkdir -p "$ASTRID_MANAGED_BIN"
  tar -xzf "$tmp/$asset" -C "$tmp"
  found=""
  for d in "$tmp"/* "$tmp"; do
    [ -d "$d" ] || continue
    if [ -x "$d/astrid" ] && [ -x "$d/astrid-daemon" ]; then found="$d"; break; fi
  done
  [ -n "$found" ] || die "archive missing astrid + astrid-daemon"
  for b in astrid astrid-daemon astrid-build; do
    [ -x "$found/$b" ] && cp -f "$found/$b" "$ASTRID_MANAGED_BIN/$b" && chmod 755 "$ASTRID_MANAGED_BIN/$b"
  done
  export PATH="${ASTRID_MANAGED_BIN}:${PATH}"
  ASTRID="$ASTRID_MANAGED_BIN/astrid"
  export ASTRID_BIN_ROOT="$ASTRID_MANAGED_BIN"

  # PATH for future shells (once)
  case "$(uname -s 2>/dev/null)" in Darwin) path_rc="$HOME/.zprofile" ;; *) path_rc="$HOME/.profile" ;; esac
  if [ -n "$path_rc" ] && ! grep -qF '.astrid/bin' "$path_rc" 2>/dev/null; then
    printf '\n# Astrid CLI\nexport PATH="%s:$PATH"\n' "$ASTRID_MANAGED_BIN" >> "$path_rc"
    v "added PATH to $path_rc"
  fi
  trap - EXIT
  rm -rf "$tmp" 2>/dev/null || true
  ok "installed $($ASTRID --version 2>/dev/null | head -n1) → $ASTRID_MANAGED_BIN"
}

ensure_cli() {
  if [ -n "$BIN_ROOT" ]; then
    has_pair "$BIN_ROOT" || die "--bin-root missing astrid + astrid-daemon"
    ASTRID="$BIN_ROOT/astrid"
    export ASTRID_BIN_ROOT="$BIN_ROOT"
    ok "using $BIN_ROOT"
    return 0
  fi
  if [ -n "${ASTRID_BIN:-}" ] && [ -x "$ASTRID_BIN" ]; then
    ASTRID="$ASTRID_BIN"
    ok "using ASTRID_BIN=$ASTRID"
    return 0
  fi

  if has_pair "$ASTRID_MANAGED_BIN"; then
    export PATH="${ASTRID_MANAGED_BIN}:${PATH}"
    ASTRID="$ASTRID_MANAGED_BIN/astrid"
    export ASTRID_BIN_ROOT="$ASTRID_MANAGED_BIN"
  elif have astrid; then
    ASTRID="$(command -v astrid)"
  else
    ASTRID=""
  fi

  if [ -n "$ASTRID" ] && [ -x "$ASTRID" ]; then
    step "CLI present — astrid update"
    if "$ASTRID" update -y >/dev/null 2>&1; then
      if has_pair "$ASTRID_MANAGED_BIN"; then
        export PATH="${ASTRID_MANAGED_BIN}:${PATH}"
        ASTRID="$ASTRID_MANAGED_BIN/astrid"
      elif have astrid; then
        ASTRID="$(command -v astrid)"
      fi
      ok "$($ASTRID --version 2>/dev/null | head -n1)"
    else
      ok "kept $($ASTRID --version 2>/dev/null | head -n1) (update no-op or deferred)"
    fi
    return 0
  fi

  step "CLI missing — install from GitHub Releases"
  if install_from_github; then return 0; fi
  if [ "$NO_BREW" -eq 0 ] && have brew; then
    step "fallback — Homebrew"
    brew tap "$BREW_TAP" >/dev/null 2>&1 || true
    brew install "${BREW_TAP}/${BREW_FORMULA}" >/dev/null 2>&1 \
      || brew install "$BREW_FORMULA" >/dev/null 2>&1 \
      || die "Homebrew install failed"
    ASTRID="$(command -v astrid)" || die "brew install left no astrid on PATH"
    ok "$($ASTRID --version 2>/dev/null | head -n1)"
    return 0
  fi
  die "could not install Astrid (releases then brew)"
}

ensure_base() {
  [ "$SKIP_INIT" -eq 1 ] && return 0
  home="${ASTRID_HOME:-$HOME/.astrid}"
  if [ -d "$home/home/default" ] || [ -f "$home/home/default/.config/distro.lock" ] \
    || [ -f "$home/home/default/.config/Distro.lock" ] || [ -f "$home/config.toml" ]; then
    step "base home present"
    ok "$home"
    return 0
  fi
  step "base home — astrid init -y"
  if "$ASTRID" init -y >/dev/null 2>&1; then
    ok "initialized"
  else
    say "  ! init reported an error (run: astrid doctor)"
  fi
}

detect_hosts() {
  hosts=""
  if [ "$BASE_ONLY" -eq 1 ]; then printf '%s' ""; return 0; fi
  if [ "$ALL_HOSTS" -eq 1 ]; then printf '%s' "claude grok codex"; return 0; fi
  if [ -n "$REQUESTED_HOSTS" ]; then printf '%s' "$REQUESTED_HOSTS"; return 0; fi
  if have claude || [ -x "${HOME}/.claude/local/claude" ] || [ -d "${HOME}/.claude" ]; then
    hosts="${hosts} claude"
  fi
  if have grok || [ -d "${HOME}/.grok" ]; then hosts="${hosts} grok"; fi
  if have codex || [ -d "${HOME}/.codex" ]; then hosts="${hosts} codex"; fi
  printf '%s' "$hosts"
}

distro_url() {
  printf '%s/%s.toml\n' "$DISTRO_BASE" "$1"
}

principal_for() {
  case "$1" in
    claude) printf 'claude-code\n' ;;
    grok)   printf 'grok-code\n' ;;
    codex)  printf 'codex-code\n' ;;
  esac
}

plugin_cmd() {
  case "$1" in
    claude)
      printf 'claude plugin marketplace add %s && claude plugin install astrid@astrid-oracles\n' "$ORACLES_REPO"
      ;;
    grok)
      printf 'grok plugin marketplace add %s && grok plugin install astrid@astrid-oracles\n' "$ORACLES_REPO"
      ;;
    codex)
      printf 'codex plugin marketplace add %s && codex plugin install astrid@astrid-oracles\n' "$ORACLES_REPO"
      ;;
  esac
}

pretty() {
  case "$1" in
    claude) printf 'Claude Code\n' ;;
    grok)   printf 'Grok Build\n' ;;
    codex)  printf 'Codex\n' ;;
  esac
}

# Explicit --host: provision capsules now (Astrid's path). Default re-run does not.
provision_host() {
  host="$1"
  p="$(principal_for "$host")"
  d="$(distro_url "$host")"
  step "capsules for $(pretty "$host") → $p"
  if ! "$ASTRID" init --distro "$d" -y >/dev/null 2>&1; then
    v "default init soft-failed"
  fi
  if "$ASTRID" init --distro "$d" --principal "$p" -y >/dev/null 2>&1; then
    ok "distro applied ($p)"
  else
    say "  ! init failed for $p — export GH_TOKEN=\$(gh auth token) and re-run with --host $host"
  fi
}

# ---------------------------------------------------------------------------
main() {
  say "Astrid"
  say "  install or upgrade the CLI, ensure a bootable home,"
  say "  then point host apps at their plugins (capsules come later)."
  say ""

  ensure_cli
  ensure_base

  hosts="$(detect_hosts)"
  # shellcheck disable=SC2086
  set -- $hosts

  if [ "$#" -eq 0 ]; then
    step "no coding hosts detected"
    ok "base Astrid is enough — open Claude / Grok / Codex later and re-run, or use --host"
  else
    step "hosts: $*"
    for h in "$@"; do
      cmd="$(plugin_cmd "$h")"
      say "  $(pretty "$h") plugin:"
      say "    $cmd"
    done

    # Only when user asked --host / --all: run distro init now.
    if [ -n "$REQUESTED_HOSTS" ] || [ "$ALL_HOSTS" -eq 1 ]; then
      say ""
      step "provisioning capsules (--host/--all)"
      for h in "$@"; do
        provision_host "$h"
      done
    else
      say "  capsules: open the host app — SessionStart doctor offers astrid init"
      say "            or re-run with --host claude (etc.) to provision now"
    fi
  fi

  say ""
  say "Done.  astrid doctor   ·   https://astridos.org/start/"
  say "Upgrade anytime: curl -fsSL https://astridos.org/install.sh | sh"
}

main
