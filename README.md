# Astrid Oracles

External coding runtimes bound into **Astrid**. One backend; host adapters
only where the host protocol forces it.

## Architecture

```
                    ┌─────────────────────────────────────┐
  Claude / Grok /   │  plugins/{claude,grok,codex}        │
  Codex plugins     │    thin wrappers → plugins/common   │
                    └──────────────┬──────────────────────┘
                                   │ MCP (mcp__astrid__*)
                                   ▼
                    ┌─────────────────────────────────────┐
                    │  astrid-mcp  (oracle-broker)          │
                    │  OracleIdentity::ASTRID — one wire id │
                    └─────────────────────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
     claude-install         (grok: broker only)   codex-install
     HostProvisioner                              HostProvisioner
              │                                         │
              ▼                                         ▼
        claude-runner                              codex-runner
        (claude -p)                                (codex exec)
```

| Crate | Shared? | Role |
|-------|---------|------|
| `oracle-core` | yes | `Host`, `OracleIdentity` |
| `oracle-broker` | yes | MCP broker implementation |
| `oracle-host` | yes | `PrincipalId`, atomic fs, `HostProvisioner`, topics |
| `astrid-mcp` | yes | sole broker capsule |
| `claude-install` / `codex-install` | thin | host home layouts on shared install loop |
| `claude-runner` / `codex-runner` | host protocol | process supervision / exec |

## Plugins

```
plugins/
  common/bin/     # astrid-up, doctor, install, resolve (shared)
  claude/bin/     # thin wrappers + Claude-only statusline
  grok/bin/       # thin wrappers
  codex/bin/      # Codex-specific up (hooks surface differs)
```

## Distros

```bash
astrid init --distro ./distros/claude.toml --principal claude-code
astrid init --distro ./distros/grok.toml   --principal grok-code
astrid init --distro ./distros/codex.toml  --principal codex-code
```

## Build

```bash
cargo test -p oracle-core -p oracle-broker -p oracle-host --lib
cargo build -p astrid-mcp -p claude-install -p claude-runner \
            -p codex-install -p codex-runner
```

## License

MIT OR Apache-2.0
