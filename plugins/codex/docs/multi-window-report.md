# Multiple Codex Windows With Astrid

## Question

What happens when multiple Codex CLI windows run with the Astrid plugin
enabled?

## Current Behavior

- Each Codex window is a separate Codex process/thread runner.
- Each window loads the enabled plugin and its bundled hooks independently.
- On `SessionStart`, each window runs:
  `astrid-plugin/bin/astrid-up codex session-start`.
- The current shim converges every window onto one Astrid agent principal named
  `codex-code` by default.
- Hook events are routed through `astrid-up codex hook <event>`.
- Current installed Astrid CLI does not expose `astrid emit`, so hook payloads
  are consumed and dropped by the compatibility shim until the native emitter
  lands.

This means multiple windows currently behave like many Codex sessions sharing
one Astrid identity, not like distinct Astrid-managed agents.

## Implemented Baseline

Astrid keeps `codex-code` as the default plugin principal and adds a
session discriminator:

- `codex` records `codex.session.<session_id>` metadata in KV.
- Session IDs used in topics must be single topic segments; dotted IDs are
  rejected.
- Headless Codex runs receive `ASTRID_PRINCIPAL_ID`, `ASTRID_SESSION_ID`,
  optional `ASTRID_WORKSPACE_ID`, and a per-session `ASTRID_HOOK_TOKEN`.
- Plugin hook events now target `codex.v1.hook.<session_id>.<event>`.
- Hook payloads are wrapped in a compact attribution envelope with base64 raw
  payload, process ids, CWD, principal, session, workspace, and token fields.
- The capsule audits hook events as verified only when the claimed hook token
  matches the registered session token.

The plugin remains fail-open by default for local development. Set
`ASTRID_SIBYL_HOOK_FAIL_CLOSED=1` when running in managed mode and an Astrid
emit failure should block the hook.

## Documented Codex Hook Semantics

- Hooks are enabled by default unless `[features].hooks = false`.
- Plugin-bundled hooks load alongside user, project, and inline config hooks.
- Matching hooks from multiple files all run.
- Multiple matching command hooks for the same event are launched concurrently.
- Non-managed command hooks must be reviewed and trusted by Codex before they
  run.
- `SessionStart` runs at thread scope.
- `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`,
  `PostCompact`, `UserPromptSubmit`, `SubagentStop`, and `Stop` run at turn
  scope.
- `SubagentStart` runs at subagent-start scope.

## Risks

### First-Run Principal Race

If two windows start before the `codex-code` principal exists, both can observe
missing state and both can try to create it. One should win; the loser may see a
duplicate-agent or daemon error.

The shim needs an idempotent create path:

1. Acquire a short host lock.
2. Re-check `astrid agent show codex-code`.
3. Create if still missing.
4. Treat "already exists" as success.

### Identity Collapse

All windows currently share the `codex-code` principal. That is acceptable for a
single personal Codex identity, but it is not enough for "many Codex agents" if
Astrid needs separate quotas, budgets, capabilities, audit views, memory, or
kill switches per window.

Astrid needs a second identity axis:

- Principal: durable governed identity, for example `codex-code`.
- Session/instance: per Codex window/thread/turn, for example
  `codex-session-<id>`.
- Workspace: mounted project/worktree/CWD context. This is not an identity
  boundary.

If the user explicitly starts multiple autonomous Codex agents, Astrid should
also support distinct principals such as `codex-main`, `codex-reviewer`, and
`codex-fixer`.

### Event Attribution

Once `astrid emit` exists, every hook event must include enough attribution to
separate windows:

- Astrid principal id.
- Codex thread id, if available.
- Codex turn id, if available.
- Process id.
- CWD/project root.
- Hook event name.
- Tool name or matcher target, when applicable.
- Monotonic local sequence number.
- Timestamp.

Without these fields, concurrent hook events from different windows will
collapse into the same `codex.v1.hook.*` stream and become hard to audit or
route.

Hook events should carry both routing and audit attribution:

- Topic discriminator: `codex.v1.hook.<session_id>.<event>`.
- Payload fields: `principal_id`, `session_id`, `workspace_id`, `pid`, `ppid`,
  raw payload, token, sequence/timestamp when available.

### Ordering

Codex runs multiple matching command hooks concurrently, and separate windows
run independently. Astrid must treat hook events as unordered and at-least-once.
Policy and audit capsules should be idempotent.

### Blocking Policy Gap

The current compatibility hook is fail-open for hook forwarding because
`astrid emit` does not exist yet. That is correct for developer UX, but it means
Astrid is not yet enforcing Codex tool policy through native hooks.

Blocking enforcement needs a native hook bridge that can:

- Read the Codex hook payload from stdin.
- Publish a request onto Astrid's bus.
- Wait within Codex's hook timeout.
- Convert Astrid's decision back into Codex's hook output contract.
- Fail closed only for managed/policy mode; fail open for local developer mode
  if configured.

### Daemon Availability

The startup hook currently attempts `astrid status`, then best-effort
`astrid start`, then agent lookup/create. If the daemon is unavailable, the
startup hook can fail before the window is registered.

The recommended behavior is mode-dependent:

- Local plugin mode: fail open, record a degraded local diagnostic, do not break
  Codex startup.
- Managed Astrid mode: fail closed if policy requires Astrid supervision.

## Better Architecture

### Level 1: Safe Local Plugin

- Keep one `codex-code` principal by default.
- Add a lock around principal creation.
- Generate a per-window session id at `SessionStart`.
- Persist the session id somewhere stable for that Codex process.
- Include the session id in all hook events.
- Keep hook forwarding fail-open until native emit/enforcement exists.

Status: partially implemented. Principal creation is locked, hook topics are
session-scoped, and the shim derives a session from `ASTRID_SESSION_ID`, Codex
thread fields when available, or process ancestry as a degraded fallback.

### Level 2: Astrid-Managed Codex Profile

- Add `astrid codex up`.
- It starts Astrid, ensures the principal, relinks Codex config/hooks/MCP, then
  launches `codex --profile codex`.
- The generated profile selects Astrid-managed permissions, hooks, MCP, memory,
  approval reviewer, and telemetry.
- The launcher injects `ASTRID_PRINCIPAL_ID`, `ASTRID_SESSION_ID`,
  `ASTRID_WORKSPACE_ID`, and `ASTRID_HOOK_TOKEN`.

### Level 3: Native Hook Translator

- Ship a native `astrid-codex-hook` or general `astrid-emit` host binary.
- The plugin hooks call this binary directly.
- The binary preserves payloads as opaque JSON and maps them to
  `codex.v1.hook.<session_id>.<event>`.
- Policy capsules can return decisions for blocking events.

### Level 4: App-Server Control Plane

- Use `codex app-server` for deep integration rather than only hook shims.
- Astrid connects as a JSON-RPC client and observes thread/turn/item events.
- Multiple windows become multiple app-server clients or threads with explicit
  IDs.
- Astrid dashboards can display active Codex windows, approvals, tool calls,
  diffs, budgets, and status.

### Level 5: Workspace Isolation Policy

- Keep principal identity stable across CWDs.
- Record `workspace_id`/mount context on every session.
- Let policy decide what happens when two sessions target the same workspace:
  shared-write with audit, per-session worktree, per-session VFS overlay, or
  observe-only.
- Current Astrid VFS overlays are per principal, so true same-principal
  workspace isolation needs a core follow-up keyed by
  `(principal_id, workspace_id, session_id)`.

## Recommendation

For the current plugin, treat multiple windows as many Codex sessions under one
`codex-code` principal. Before claiming true multi-agent management, add:

1. A locked/idempotent principal bootstrap.
2. Per-window session ids.
3. Hook event attribution.
4. A native emit/hook translator.
5. App-server integration for rich lifecycle control.
6. A workspace isolation policy for same-principal concurrent writes.

That preserves convenient local Codex startup while giving Astrid the control
plane it needs for safe extension.
