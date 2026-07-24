# Unicity AOS for Grok Build

This plugin connects Grok Build to Unicity AOS as the scoped `grok-code`
principal. It registers the public `aos` MCP server, a session-start readiness
doctor, operator commands, and capsule-authoring skills. On a blank slate, the
doctor may run the host-scoped installer before reporting readiness. Commands
are available by their unqualified names; if another plugin defines the same
command, use the host-qualified form such as `/unicity-aos:doctor`.

Install and provision the product and Grok adapter explicitly:

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/unicity-aos/oracles/main/install.sh \
  | sh -s -- --host grok
```

Or install the local plugin during development:

```sh
grok plugin install /path/to/oracles/plugins/grok --trust
grok plugin enable unicity-aos
```

`.mcp.json` executes `bin/aos-up --principal grok-code`. On a blank slate, that
wrapper starts provisioning only the Grok pack in the background and waits for
its committed receipt; the SessionStart doctor shares the same idempotent
installer. Grok can retry the MCP connection while provisioning continues. Once
ready, the wrapper delegates to
`aos --principal grok-code mcp serve`. Existing installations take the ready
path without re-entering the installer.
Before that handoff, it restores the project directory from which Grok launched
the adapter, keeping AOS product state separate from the agent workspace.

Visible tools use `mcp__aos__*`. The internal broker capsule remains
`aos-mcp`, and its stable runtime topics remain `astrid.v1.*`.
