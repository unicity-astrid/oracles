# Unicity AOS for Codex

This plugin connects Codex to Unicity AOS as the `codex-code` principal and
teaches a fresh session how to operate and build on the OS.

## What loads

On installation, Codex reads `.codex-plugin/plugin.json` and discovers:

- `skills/unicity-aos` for the operating-system and principal boundary;
- `skills/capsule-forge` for a complete capsule authoring workflow;
- `skills/meta-harness` for building a governed user-space meta-harness on AOS;
- `.mcp.json`, which starts `aos --principal codex-code mcp serve`; and
- session and tool hooks under `hooks/hooks.json`.

Skill metadata is available for routing. Codex reads a skill's complete
`SKILL.md` only when the user names it or the request matches its description.
The MCP server independently exposes the tool surface granted to `codex-code` as
`mcp__aos__*`. Installing the plugin therefore supplies both the knowledge of
how to build and the governed tools for inspecting or changing the live OS.

## Fresh-session path

For a request such as “build a Slack meta-harness on AOS,” a fresh Codex session
should:

1. Load `unicity-aos` and recognize AOS as the operating system, not the
   meta-harness.
2. Load `meta-harness` for worker scope, lifecycle, capability-gap, quarantine,
   evaluation, approval, and rollback rules.
3. Load `capsule-forge` when a missing connector or capability requires code.
4. Inspect the actual `mcp__aos__*` tools, capsules, and WIT contracts.
5. Reuse, compose, or configure installed capabilities before authoring a new
   capsule.
6. Build and test the smallest missing capability, then leave promotion to AOS
   policy and operator approval.

The static skills are sufficient to explain the architecture and author a
capsule from zero, even if AOS is temporarily offline. Live discovery,
installation, grants, and supervision still require the MCP server and the
relevant AOS capsules. If AOS does not yet expose a durable worker API, Codex
must report that gap; a shell process is not a substitute for an agent.

The `capsule-forge` instructions are vendored from the Forge capsule in the AOS
Community Edition source. Host-specific text may explain MCP discovery, but the
capsule guide remains the authority for SDK, manifest, WIT, secret, toolchain,
and packaging behavior.

After installing or updating the plugin, start a new Codex thread so its skills,
hooks, and MCP tools are discovered together.
