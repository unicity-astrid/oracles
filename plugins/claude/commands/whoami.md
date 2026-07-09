---
description: Show the Astrid principal this session acts as, plus its capabilities and quota.
allowed-tools: Bash(astrid agent show:*), Bash(astrid caps show:*), Bash(astrid quota show:*)
---

Report the Astrid identity and mandate backing THIS session.

The session acts as its own scoped principal (decoupled from the operator's
active CLI context). Claude Code exports that principal to plugin subprocesses
as `CLAUDE_PLUGIN_OPTION_PRINCIPAL`, so the commands below target it directly;
if it is unset they fall back to the active CLI context.

Principal:

!`astrid agent show ${CLAUDE_PLUGIN_OPTION_PRINCIPAL:-${CLAUDE_PLUGIN_OPTION_principal:-}} 2>/dev/null || echo "(unavailable — is the astrid CLI installed?)"`

Capabilities held by that principal:

!`astrid caps show ${CLAUDE_PLUGIN_OPTION_PRINCIPAL:-${CLAUDE_PLUGIN_OPTION_principal:-}} 2>/dev/null || echo "(unavailable — the daemon may be down)"`

Quota / budget:

!`astrid quota show ${CLAUDE_PLUGIN_OPTION_PRINCIPAL:-${CLAUDE_PLUGIN_OPTION_principal:-}} 2>/dev/null || echo "(unavailable — the daemon may be down)"`

Summarize in a few lines: who this session acts as, what it is allowed to do, and any budget limits. Note that this is a least-authority `agent`-group principal (self-scoped), not the admin `default`. If a section is unavailable, say so and note that `astrid start` brings the daemon up.
