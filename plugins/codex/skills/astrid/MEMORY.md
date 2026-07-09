# Astrid Codex Memory

This file captures durable Codex orientation for Astrid OS work. Keep it short,
accurate, and updated when the architecture or workflow changes.

## Operating Model

- Astrid OS should manage Codex as one governed agent among many.
- Codex integration should mirror the Claude/Sage integration where the runtime
  contracts match, but Codex-specific process behavior should not be forced into
  Claude-shaped assumptions.
- Always read the relevant Astrid book and handbook chapters before changing
  architecture, capsule boundaries, build flow, packaging, or agent behavior.
- Treat this worktree as shared. Claude may be editing concurrently; do not
  revert or overwrite live work unless Joshua explicitly asks.

## Capsule Rules

- Astrid uses capsule roots, not a central capsule workspace.
- A capsule root has its own `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
  `.cargo/config.toml`, `Capsule.toml`, README, licenses, and `src/`.
- `Capsule.toml` belongs beside the capsule root `Cargo.toml`. It is the
  installer/build contract consumed by `astrid build`.
- Capsule directories follow `capsules/astrid-capsule-<name>/`.
- GitHub repo names normally drop the `astrid-` prefix, for example
  `capsule-<name>`.
- Capsules target `wasm32-unknown-unknown`; root `.cargo/config.toml` should set
  that target and the custom getrandom backend cfg used by Astrid.

## Architecture Rules

- Keep the kernel dumb. Business logic, agent loops, model/provider behavior,
  provider adapters, and protocol bridges belong in capsules.
- Declare authority explicitly in `Capsule.toml`, including publish/subscribe
  topics, KV access, host capabilities, process access, network access, and
  filesystem mounts.
- Preserve principal isolation. Treat principal IDs, session IDs, IPC payloads,
  and MCP/tool payloads as untrusted until validated.
- Use `astrid-sdk` and established capsule patterns before inventing new local
  abstractions.
- Prefer bounded, auditable host calls and event publication over hidden runtime
  side effects.

## Current Astrid Shape

- `codex` is the governed Codex runner capsule.
- `codex-install` is the bootstrap/relink capsule for Codex config, hooks, and
  plugin linkage.
- `codex-mcp` is the Codex-facing MCP/tool broker. It serves
  `astrid.v1.request.mcp.tools.list`, dispatches
  `astrid.v1.request.mcp.tool.call` through `tool.v1.execute.<name>`, and
  answers `hook.v1.event.before_tool_call` with policy verdicts.
- The Codex plugin `.mcp.json` enters through
  `astrid-up --principal codex-code`, mirroring Sage's `claude-code` principal
  shape. The shim defaults blank or unexpanded principal config to `codex-code`,
  boots or reuses an ephemeral Astrid daemon, provisions the scoped principal
  in the `codex` family group, then execs
  `astrid mcp serve --principal <principal>`.
- `astrid-plugin` is the Codex plugin with skill, hook, and MCP defaults.
- Codex execution currently uses bounded `codex exec` turns because Codex does
  not have the same long-lived `claude -p` process contract.
- Multiple Codex windows should normally share the same durable principal
  (`codex-code`) and differ by `session_id`. Split principals only for real
  policy, secret, quota, tenant, memory, or kill-switch boundaries.
- Session IDs, hook event names, workspace IDs, correlation IDs, and call IDs
  used inside IPC topics must be single topic segments. Do not allow dots in
  those values.
- Workspace/CWD/worktree context belongs in `workspace_id` or mount metadata,
  not in the principal. Current core VFS overlays are per principal, so
  same-principal per-session workspace isolation needs a core follow-up.

## Codex Surfaces Astrid Should Manage

- `CODEX_HOME` defaults to `~/.codex`; the important durable files are
  `config.toml`, profile files such as `astrid.config.toml`, `hooks.json`,
  plugin state, MCP config, logs, history, and generated memories.
- Astrid should be the source of truth for generated Codex config. User edits
  can still exist, but Astrid-owned blocks need markers, schema versions, and a
  relink path from capsule KV to disk.
- Project `.codex/config.toml` and `.codex/hooks.json` only load for trusted
  projects. User/global config and managed config remain separate.
- Codex hooks are enabled by default and can be disabled with
  `[features].hooks = false`. Managed requirements can force hooks on and can
  allow only managed hooks.
- Supported hook events are `SessionStart`, `UserPromptSubmit`, `PreToolUse`,
  `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`,
  `SubagentStart`, `SubagentStop`, and `Stop`.
- Hook matchers filter tool names for tool/permission events, compaction
  trigger for compact events, start source for session start, and subagent type
  for subagent events. `UserPromptSubmit` and `Stop` ignore matchers.
- Plugin hooks, user hooks, project hooks, and inline config hooks all load
  together. Non-managed command hooks must be trusted by Codex unless an
  automation explicitly bypasses hook trust.
- MCP server definitions live in `config.toml`; plugin-provided MCP servers can
  be controlled through `plugins.<plugin>.mcp_servers.*` policy.
- `astrid mcp serve` is the native MCP ingress Codex should call. It talks to
  the sanitized `astrid.v1.request.mcp.*` surface; MCP brokers should hide
  direct `tool.v1.*` fan-out from external clients.
- Permission profiles are the modern way to manage filesystem/network access.
  Prefer them over ad hoc `sandbox_mode` plus `sandbox_workspace_write` when
  Astrid needs reusable principal policies.
- `requirements.toml` and cloud/MDM managed config can enforce approval modes,
  approval reviewers, permission profiles, sandbox modes, web search, MCP
  allowlists, managed hooks, feature flags, and guardian auto-review policy.
- Codex memories are a generated local recall layer, not a policy source.
  Required project rules belong in checked-in docs, `AGENTS.md`, Astrid policy,
  or Astrid-managed config.
- Codex app-server is the preferred deep-control interface when Astrid needs
  rich clients, conversation history, approval flow, and streamed events. It is
  JSON-RPC over stdio, websocket, or Unix socket and supports thread/turn
  lifecycle APIs.
- `codex exec --json` remains useful for bounded automation; its JSONL stream
  exposes thread, turn, item, and error events that can be mirrored onto the
  Astrid bus.
- The Astrid runner mirrors bounded `codex exec --json` records onto
  `codex.v1.event.<session_id>.codex.<event-type>` topics by default.

## Codex Hook Translation Plan

- On plugin `SessionStart`, call Astrid first to ensure the configured
  principal, defaulting to `codex-code`, and publish a session-start event.
- Native Codex hooks should be translated to
  `codex.v1.hook.<session_id>.<event>` topics with the original Codex hook
  payload preserved as opaque base64 inside an attribution envelope.
- Hook envelopes should include `principal_id`, `session_id`, `workspace_id`
  when known, process IDs, CWD, token, and raw payload. Treat those as claims
  until validated by a session token or a future kernel-issued capability.
- Current installed Astrid CLI has `agent` management but no native
  `astrid emit` or `astrid codex` namespace yet. The plugin `astrid-up` shim is
  the compatibility layer until those commands land.
- Capsules should subscribe to those topics to provide policy, audit, memory,
  budget, telemetry, notification, and workflow extensions.
- For blocking hooks, Astrid policy capsules should return a deterministic
  allow/prompt/deny decision using Codex's documented hook output contract.
- Keep hook scripts as thin shims. Policy belongs in Astrid capsules, not in
  shell scripts under `~/.codex`.

## High-Leverage Integration Ideas

- `astrid codex doctor`: checks Codex CLI version, hook trust state, active
  plugin, generated config hash, MCP health, app-server availability, and
  principal isolation.
- Astrid-managed `codex.config.toml` profile: selects permission profile,
  approval reviewer, MCP policy, hooks feature, memory policy, and app-server
  defaults for the current principal.
- Hook translator capsule: maps every Codex hook event into Astrid-native
  events and maps Astrid policy responses back into Codex hook output.
- App-server capsule or native shim: runs `codex app-server` behind Astrid so
  dashboards can observe turns, approvals, tool calls, diffs, and token usage.
- Event-sourced Codex audit: mirror `codex exec --json`, OTel events, and hook
  events into per-principal Astrid audit chains.
- Tool marketplace bridge: Astrid capsules that declare `#[astrid::tool]`
  become MCP tools surfaced to Codex through the Astrid MCP server, with
  per-principal capability gates.
- Managed fleet mode: generate or install `requirements.toml`/managed hooks so
  enterprises can make Astrid policy non-bypassable.
- Memory bridge: Codex memory summaries can inform Astrid memory, but Astrid
  should keep policy and identity in explicit capsules/KV.

## Remaining Astrid Work

- Promote the plugin `astrid-up codex up` compatibility launcher into a native
  `astrid codex up` command.
- Expand Codex profile generation with approval reviewer, shell environment
  policy, and telemetry defaults.
- Add app-server integration for long-lived Codex control. `codex exec --json`
  remains useful for bounded turns, but app-server is the richer session/turn
  control plane.
- Add managed-config support for non-bypassable hooks, MCP allowlists,
  approval policy, and permission profiles in managed deployments.
- Add core VFS/workspace policy for same-principal concurrent Codex sessions.

## Rust Standards

- Use current crate versions when adding dependencies, and verify with Cargo.
- Run `cargo fmt --check` and `cargo check` from each capsule root after edits.
- Do not rely on a parent workspace lockfile for capsules; each capsule root owns
  its lockfile.
