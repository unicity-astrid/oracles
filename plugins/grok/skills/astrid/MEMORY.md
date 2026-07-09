# Mimir memory

- Product: Mimir = Grok Build on Astrid (peer of Sage/Claude and Sibyl/Codex).
- Default principal: `grok-code` in group `grok`.
- Plugin path: `astrid-plugin/` with `.grok-plugin/plugin.json`.
- MCP: `bin/astrid-up --principal grok-code` → `astrid mcp serve`.
- Broker: `mimir-mcp` answering standard `astrid.v1.request.mcp.*`.
- Binary resolution: `ASTRID_BIN`/`ASTRID_DAEMON` → `ASTRID_BIN_ROOT` → `.mcp.json` → nearby `core/target/{debug,release}` → installs/PATH. Fail-closed on CLI/daemon binary mismatch.
- Grok SessionStart hooks do not inject model context the way Claude's `additionalContext` does — identity lives in this skill + `/doctor`.
