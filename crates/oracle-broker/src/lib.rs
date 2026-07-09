//! Shared MCP broker for Astrid-governed oracle products.
//!
//! Astrid oracle identity is injected via
//! [`install`](profile::install). The astrid-mcp capsule call `install` then
//! forward host interceptors to the handlers re-exported here.
//!
//! The live execution door is the product-neutral `astrid.v1.request.mcp.*`
//! surface. Product-local topics (`{product}.v1.tools.*`, audit) are derived
//! from the installed [`oracle_core::OracleIdentity`].

#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(unreachable_pub)]
#![warn(missing_docs)]

mod approval;
mod broker;
mod cache;
mod discovery;
mod execute;
mod grant_decision;
mod hook_gate;
mod policy;
mod profile;

pub use oracle_core::{Host, HostProfile, OracleIdentity};
pub use profile::{install, install_astrid};

/// Capsule entry points — product-agnostic once [`install`] has run.
pub mod handlers {
    //! Interceptor-shaped handlers for the astrid-mcp capsule.

    use astrid_sdk::prelude::*;

    /// `astrid.v1.tools.describe` — assemble and publish the tool list.
    pub fn describe_tools(_payload: serde_json::Value) -> Result<(), SysError> {
        crate::discovery::describe_tools();
        Ok(())
    }

    /// `tool.v1.response.describe.*` — event-driven cache merge.
    pub fn collect_tool_descriptors(payload: serde_json::Value) -> Result<(), SysError> {
        crate::discovery::collect_tool_descriptors(payload);
        Ok(())
    }

    /// `astrid.v1.capsules_loaded` — invalidate / rebuild tool cache.
    pub fn handle_capsules_changed(payload: serde_json::Value) -> Result<(), SysError> {
        crate::discovery::on_capsules_loaded(payload);
        Ok(())
    }

    /// `astrid.v1.request.mcp.tools.list` — broker list front door.
    pub fn handle_mcp_list(payload: serde_json::Value) -> Result<(), SysError> {
        crate::broker::handle_mcp_list(payload)
    }

    /// `astrid.v1.request.mcp.tool.call` — broker call front door.
    pub fn handle_mcp_call(payload: serde_json::Value) -> Result<(), SysError> {
        crate::broker::handle_mcp_call(payload)
    }

    /// `astrid.v1.request.mcp.approval.respond` — approval bridge.
    pub fn handle_mcp_approval(payload: serde_json::Value) -> Result<(), SysError> {
        crate::approval::handle_mcp_approval(payload)
    }

    /// `astrid.v1.request.mcp.ingress.respond` — ingress consent bridge.
    pub fn handle_mcp_ingress_respond(payload: serde_json::Value) -> Result<(), SysError> {
        crate::approval::handle_mcp_ingress_respond(payload)
    }

    /// `astrid.v1.request.mcp.grant.respond` — capsule-grant consent bridge.
    pub fn handle_mcp_grant_respond(payload: serde_json::Value) -> Result<(), SysError> {
        crate::approval::handle_mcp_grant_respond(payload)
    }

    /// `hook.v1.event.before_tool_call` — native-tool verdict responder.
    pub fn handle_before_tool_call(payload: serde_json::Value) -> Result<(), SysError> {
        crate::hook_gate::handle_before_tool_call(payload)
    }
}
