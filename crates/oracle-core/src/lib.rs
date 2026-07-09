//! Shared identity for Astrid oracles.
//!
//! An **oracle** is an external coding runtime (Claude Code, Grok Build, Codex)
//! bound into Astrid. The backend is always Astrid — one MCP namespace, one
//! broker capsule. Hosts differ only where the host product forces it
//! (plugin hooks, principal family, optional supervisor).
//!
//! Mythological product brands (Sage / Mimir / Sibyl) are retired: the legal
//! and product surface is **Astrid**, not a co-branded third-party name.

#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(unreachable_pub)]
#![warn(missing_docs)]

mod host;
mod identity;
mod newtypes;

pub use host::{Host, HostProfile};
pub use identity::OracleIdentity;
pub use newtypes::{
    AuditTopicPrefix, CapsuleName, DistroId, HostDisplayName, LogTag, McpNamespace, McpToolPrefix,
    PrincipalFamily, Topic,
};
