---
description: Show the Astrid principal this session acts as, plus its capabilities and quota.
---

Report the Astrid identity and mandate backing THIS session.

The session acts as its own scoped principal (decoupled from the operator's
active CLI context). Prefer the session principal marker published by
`bin/astrid-up`, then `grok-code`.

Principal:

!`PRINCIPAL="$(tr -d '[:space:]' < "${ASTRID_HOME:-$HOME/.astrid}/run/session.principal" 2>/dev/null || true)"; PRINCIPAL="${PRINCIPAL:-grok-code}"; astrid agent show "$PRINCIPAL" 2>/dev/null || echo "(unavailable — is the astrid CLI installed?)"`

Capabilities held by that principal:

!`PRINCIPAL="$(tr -d '[:space:]' < "${ASTRID_HOME:-$HOME/.astrid}/run/session.principal" 2>/dev/null || true)"; PRINCIPAL="${PRINCIPAL:-grok-code}"; astrid caps show "$PRINCIPAL" 2>/dev/null || echo "(unavailable — the daemon may be down)"`

Quota / budget:

!`PRINCIPAL="$(tr -d '[:space:]' < "${ASTRID_HOME:-$HOME/.astrid}/run/session.principal" 2>/dev/null || true)"; PRINCIPAL="${PRINCIPAL:-grok-code}"; astrid quota show "$PRINCIPAL" 2>/dev/null || echo "(unavailable — the daemon may be down)"`

Summarize in a few lines: who this session acts as, what it is allowed to do, and any budget limits. Note that this is a least-authority `grok` family principal (self-scoped), not the admin `default`. If a section is unavailable, say so and note that the plugin boots an ephemeral daemon when MCP starts.
