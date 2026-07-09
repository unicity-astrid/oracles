//! codex-install — thin [`oracle_host::HostProvisioner`] for Codex.
//!
//! Install loop, markers, ids, and atomic writes live in `oracle-host`.
//! This crate only owns the `.codex/` file bodies.

#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(unreachable_pub)]
#![warn(missing_docs)]

use astrid_sdk::prelude::*;
use oracle_core::Host;
use oracle_host::fs::write_atomic;
use oracle_host::{
    HostProvisioner, HostTopics, InstallRequest, PrincipalId, RelinkRequest, publish_complete,
    run_install, run_relink,
};

/// Artifact shape for managed `.codex/` files.
///
/// v3: SessionStart doctor prefers the Codex plugin `bin/astrid-doctor` (update
/// clocks). Core `astrid doctor` has no `--format hook` and is not a substitute.
const ARTIFACT_VERSION: u32 = 3;

struct CodexLayout;

impl HostProvisioner for CodexLayout {
    type Context = ();
    const ARTIFACT_VERSION: u32 = ARTIFACT_VERSION;

    fn topics() -> HostTopics {
        HostTopics::for_host(Host::Codex)
    }

    fn ensure_dirs(_ctx: &Self::Context) -> Result<(), SysError> {
        fs::create_dir_all("home://.codex")?;
        fs::create_dir_all("home://.codex/hooks")?;
        Ok(())
    }

    fn write_files(_ctx: &Self::Context) -> Result<(), SysError> {
        write_atomic(
            "home://.codex/config.toml",
            br#"# Astrid-managed Codex base config.
# Principal policy: `codex --profile astrid` uses astrid.config.toml

[features]
hooks = true

[mcp_servers.astrid]
command = "astrid"
args = ["mcp", "serve"]
enabled = true
required = true
default_tools_approval_mode = "prompt"
startup_timeout_sec = 20
tool_timeout_sec = 600
"#,
        )?;
        write_atomic(
            "home://.codex/astrid.config.toml",
            br#"approval_policy = "on-request"
sandbox_mode = "workspace-write"
default_permissions = ":workspace"
model_reasoning_summary = "auto"
model_verbosity = "medium"

[features]
hooks = true
"#,
        )?;
        // SessionStart doctor: plugin path first (has runtime/plugin/distro
        // update clocks + host playbook). Fall back to no-op — core `astrid
        // doctor` has no hook JSON format. Full policy hooks live in the
        // marketplace plugin (`plugins/codex/hooks/hooks.json`).
        write_atomic(
            "home://.codex/hooks.json",
            br#"{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "root=\"${PLUGIN_ROOT:-${CODEX_PLUGIN_ROOT:-}}\"; if [ -x \"$root/bin/astrid-doctor\" ]; then \"$root/bin/astrid-doctor\" --format hook; else exit 0; fi",
            "timeout": 15,
            "statusMessage": "Checking Astrid updates"
          }
        ]
      }
    ]
  }
}
"#,
        )?;
        Ok(())
    }
}

/// Codex install capsule.
#[derive(Default)]
pub struct CodexInstall;

#[capsule]
impl CodexInstall {
    /// `codex.v1.install.run`
    #[astrid::interceptor("handle_install")]
    pub fn handle_install(&self, req: InstallRequest) -> Result<(), SysError> {
        let principal = PrincipalId::parse(&req.principal_id)?;
        match run_install::<CodexLayout>(&principal, req.force, &()) {
            Ok(already) => publish_complete::<CodexLayout>(
                &principal,
                true,
                Some(already),
                None,
                "home://.codex",
            ),
            Err(err) => {
                publish_complete::<CodexLayout>(&principal, false, None, Some(err.to_string()), "")
            }
        }
    }

    /// `codex.v1.install.relink`
    #[astrid::interceptor("handle_relink")]
    pub fn handle_relink(&self, req: RelinkRequest) -> Result<(), SysError> {
        let principal = PrincipalId::parse(&req.principal_id)?;
        match run_relink::<CodexLayout>(&principal, &()) {
            Ok(()) => publish_complete::<CodexLayout>(
                &principal,
                true,
                Some(false),
                None,
                "home://.codex",
            ),
            Err(err) => {
                publish_complete::<CodexLayout>(&principal, false, None, Some(err.to_string()), "")
            }
        }
    }
}
