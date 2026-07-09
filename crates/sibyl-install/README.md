# sibyl-install

Per-principal Codex provisioner for Astrid OS.

This capsule writes `home://.codex/` config and hook declarations for the
invoking principal.

It writes:

- `config.toml` with hooks enabled and the Astrid MCP server required.
- `sibyl.config.toml` as the principal's Codex profile.
- `hooks.json` for local health/stop forwarding.
- `sibyl.requirements.toml` as a staged managed-config source body.

The install marker records an artifact version so existing principals can be
reconciled when the generated file shape changes.
