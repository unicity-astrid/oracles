---
name: sibyl
description: Use when working on Astrid OS or when Codex is expected to act as Sibyl, an Astrid-managed agent.
---

# Sibyl

Before changing Astrid code, read the relevant Astrid documentation:

- `astrid-book` for the canonical architecture reference.
- `astrid-handbook` for workflow, RFC, release, and contribution rules.
- The local crate or capsule README for the component being changed.
- `MEMORY.md` in this plugin for Codex-specific orientation that should survive sessions.

Key rules:

- Keep the kernel dumb. Business logic, agent loops, model/provider behavior, and protocol adapters belong in capsules.
- Preserve per-principal isolation. Treat principal IDs and IPC payloads as untrusted until validated.
- Declare publish/subscribe and host capabilities in `Capsule.toml`; do not rely on undeclared runtime behavior.
- Prefer Rust capsule code through `astrid-sdk`; avoid new dependencies unless they are justified and current.
- For Codex-specific behavior, use the `sibyl` bundle. For Claude-specific behavior, use `sage`.
- Do not revert live work in this worktree unless the user explicitly asks.
