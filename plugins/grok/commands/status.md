---
description: Show the Astrid daemon status — PID, uptime, connected clients, loaded capsules.
---

Report the live Astrid daemon status.

!`astrid status 2>/dev/null || echo "(unavailable — is the astrid CLI installed / is the daemon up?)"`

Summarize: is the daemon up, what PID, how many capsules are loaded, and whether this looks healthy for a governed session.
