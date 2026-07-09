# Mimir — Grok Build plugin

Turn a vanilla Grok Build session into a **governed Astrid agent**. Installing this plugin registers the Astrid MCP server (`astrid mcp serve`) so Grok discovers and calls the live Astrid capsule tool surface (filesystem, http, shell, system, skills, …), scoped to an Astrid principal — with the daemon brought up automatically and torn down when you're done.

Astrid stays agent-agnostic — it's just an MCP server on the bus. This plugin is the **Grok-specific adapter**. Sage is Claude's; Sibyl is Codex's.

## What it bundles

- **MCP server** (`.mcp.json` → `bin/astrid-up`) — ensures a daemon is running, then becomes `astrid mcp serve --principal grok-code`. Ephemeral daemon mode: self-cleans ~30s after the last client disconnects.
- **Runtime bootstrapper** (`bin/astrid-install`) — installs Astrid via Homebrew or a prebuilt release when missing.
- **SessionStart doctor** (`bin/astrid-doctor`) — readiness report. Grok does not feed Claude-style `additionalContext` into the model; use `/doctor` and the `mimir` skill for identity in context.
- **`/doctor` `/whoami` `/status` `/capsules`** — operator views via the `astrid` CLI.
- **Skills** — `mimir` (identity + Astrid rules), `forge` (author a capsule from zero).

## Principal

Default: **`grok-code`**, least-authority agent in the custom **`grok`** family group (`self:*`, `delegate:self:*`), auto-provisioned on first MCP launch. Not admin `default`. Pass `--principal default` only if you deliberately want admin authority.

## Prerequisites

Matching `astrid` + `astrid-daemon` pair from a revision that has `astrid mcp serve`. Resolution order (first hit wins):

1. `ASTRID_BIN` + `ASTRID_DAEMON`
2. `ASTRID_BIN_ROOT`
3. `ASTRID_BIN_ROOT` in this plugin's `.mcp.json` `env` block
4. Nearby worktree `core/target/{debug,release}`
5. `~/.cargo/bin`, `~/.astrid/bin`, Homebrew, PATH

If a daemon is already running from a *different* binary than the one resolved, `bin/astrid-up` refuses to attach (fail-closed).

The **mimir-mcp** broker capsule must be provisioned for the principal or `tools/list` is empty.

## Install

```sh
# pin binaries + install into Grok
./install.sh --bin-root /path/to/astrid/core/target/debug

# or raw
grok plugin install /path/to/mimir/astrid-plugin --trust
grok plugin enable mimir
```

Provision tools:

```sh
astrid init --distro /path/to/mimir/Distro.toml -y
astrid init --distro /path/to/mimir/Distro.toml --principal grok-code -y
```

## Caveats

- Tool *calls* are confused-deputy gated by `mimir-mcp` trusted ingress (same as Sage/Sibyl). First call may prompt for ingress trust.
- Native Grok tools are not yet on the Astrid bus. Only `astrid__*` MCP tools are governed through the broker.
