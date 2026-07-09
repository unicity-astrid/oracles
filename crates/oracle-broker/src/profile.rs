//! Active [`OracleIdentity`] for this broker instance.
//!
//! Thin capsule calls [`install`] once. Identity is always Astrid — hosts
//! do not get separate wire namespaces.

use std::sync::OnceLock;

use oracle_core::OracleIdentity;

static IDENTITY: OnceLock<&'static OracleIdentity> = OnceLock::new();

/// Bind this process to the Astrid oracle identity.
///
/// Idempotent. Panics only if a *different* identity pointer is installed
/// (should never happen — there is only [`OracleIdentity::ASTRID`]).
pub fn install(identity: &'static OracleIdentity) {
    match IDENTITY.set(identity) {
        Ok(()) => {}
        Err(existing) => {
            assert!(
                core::ptr::eq(existing, identity),
                "oracle-broker: identity already installed"
            );
        }
    }
}

/// Install the singleton Astrid identity if not already set.
#[inline]
pub fn install_astrid() {
    install(&OracleIdentity::ASTRID);
}

/// The installed oracle identity.
///
/// # Panics
/// If [`install`] / [`install_astrid`] has not been called.
#[inline]
#[must_use]
pub(crate) fn identity() -> &'static OracleIdentity {
    IDENTITY.get().copied().expect(
        "oracle-broker: OracleIdentity not installed; call oracle_broker::install_astrid first",
    )
}

/// Log-line tag (`astrid-mcp`).
#[inline]
#[must_use]
pub(crate) fn log_tag() -> &'static str {
    identity().log_tag.as_str()
}

/// MCP tool name prefix (`mcp__astrid__`).
#[inline]
#[must_use]
pub(crate) fn mcp_tool_prefix() -> &'static str {
    identity().mcp_tool_prefix.as_str()
}

/// Tools list publish topic.
#[inline]
#[must_use]
pub(crate) fn tools_list_topic() -> &'static str {
    identity().tools_list_topic.as_str()
}

/// Build an audit topic for `event` (single segment).
#[inline]
#[must_use]
pub(crate) fn audit_topic(event: &str) -> String {
    identity().audit_topic(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_astrid_is_idempotent() {
        install_astrid();
        install_astrid();
        assert_eq!(identity().capsule_name.as_str(), "astrid-mcp");
        assert_eq!(mcp_tool_prefix(), "mcp__astrid__");
    }
}
