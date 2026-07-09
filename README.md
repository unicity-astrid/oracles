# Astrid Oracles

External coding runtimes bound into **Astrid**. The backend is always Astrid —
one broker, one MCP namespace (`mcp__astrid__*`). Hosts only differ where the
host product forces it (hooks, principal family, optional supervisor).

| Host plugin | Principal | Distro |
|-------------|-----------|--------|
| Claude Code | `claude-code` | `distros/claude.toml` |
| Grok Build  | `grok-code`   | `distros/grok.toml` |
| Codex       | `codex-code`  | `distros/codex.toml` |

Retired brand names (Sage / Mimir / Sibyl) still resolve as aliases on
`Host::from_id` for migration. New paths use host names or plain **Astrid**.

## Layout

```
crates/
  oracle-core/      # Host, HostProfile, OracleIdentity (singleton Astrid wire id)
  oracle-broker/    # Shared MCP broker
  astrid-mcp/       # The only broker capsule
  sage/             # Claude supervisor (crate name legacy; ships Claude -p)
  sage-install/     # Claude home provisioner
  sage-completion/  # Anthropic API completion (optional)
  sibyl/            # Codex supervisor (crate name legacy)
  sibyl-install/    # Codex provisioner
plugins/
  claude/           # Claude Code host plugin
  grok/             # Grok Build host plugin
  codex/            # Codex host plugin
distros/
  claude.toml | grok.toml | codex.toml
```

## Why not three product brands?

The myth names existed to avoid looking like co-branded Claude/Grok/Codex
products. Shipping as **Astrid** is cleaner and still not a third-party
trademark claim — it's your OS, with host adapters.

## Build

```bash
cargo test -p oracle-core -p oracle-broker --lib
cargo build -p astrid-mcp
```

## License

MIT OR Apache-2.0
