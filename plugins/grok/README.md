# Astrid plugin for Grok Build

Astrid is the backend. This directory is the **Grok host adapter** (hooks + MCP wiring).

## Contents

- **MCP** (`.mcp.json`) — `astrid` server via `bin/astrid-up`
- **SessionStart doctor** (`bin/astrid-doctor` → `plugins/common`)
- **Skills** — `astrid` identity + `forge` capsule authoring

## Install

```bash
# Marketplace (when published) or local path:
grok plugin marketplace add unicity-astrid/oracles   # or a local clone
# or: grok plugin install /path/to/oracles/plugins/grok --trust
grok plugin enable astrid

# Distro (shared backend + principal):
curl -fsSL https://astridos.org/install.sh | sh -s -- --host grok
# or: astrid init --distro distros/grok.toml --principal grok-code -y
```

Plugin name is **`astrid`** (matches `.grok-plugin/marketplace.json`). Shared
shell logic lives in `plugins/common/bin/`; each host plugin ships a
self-contained copy in its own `bin/` (materialized by `scripts/sync-plugins.sh`),
so the marketplace-packaged plugin never reaches outside its own directory. The
scripts self-derive the host from its plugin-root env var. SessionStart runs
`bin/astrid-doctor` (runtime + plugin + distro update clocks, ~24h rate limit).
