#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(unreachable_pub)]
#![warn(missing_docs)]

//! sibyl-install — per-principal Codex config provisioner.

mod atomic;

use astrid_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::UNIX_EPOCH;

const COMPLETE_TOPIC: &str = "sibyl.v1.install.complete";
const STATUS_TOPIC: &str = "sibyl.v1.install.status";
const MARKER_PREFIX: &str = "sibyl.install.complete";

/// Install request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallRequest {
    /// Principal to provision.
    pub principal_id: String,
    /// Reconcile even when already installed.
    #[serde(default)]
    pub force: bool,
}

/// Relink request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct RelinkRequest {
    /// Principal to relink.
    pub principal_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallMarker {
    version: String,
    installed_at: u64,
    #[serde(default)]
    artifact_version: u32,
}

#[derive(Debug, Serialize)]
struct InstallStatus {
    principal_id: String,
    step: &'static str,
}

#[derive(Debug, Serialize)]
struct InstallComplete {
    principal_id: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    already_installed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_version: Option<u32>,
}

const ARTIFACT_VERSION: u32 = 2;

/// Sibyl install capsule.
#[derive(Default)]
pub struct SibylInstall;

#[capsule]
impl SibylInstall {
    /// Provision `home://.codex/` for a principal.
    #[astrid::interceptor("handle_install")]
    pub fn handle_install(&self, req: InstallRequest) -> Result<(), SysError> {
        let principal = sanitize_principal_id(&req.principal_id)?;
        match run_install(&principal, req.force) {
            Ok(already_installed) => {
                publish_complete(&principal, true, Some(already_installed), None)
            }
            Err(err) => publish_complete(&principal, false, None, Some(err.to_string())),
        }
    }

    /// Rewrite Codex config and hooks for a principal without changing marker state.
    #[astrid::interceptor("handle_relink")]
    pub fn handle_relink(&self, req: RelinkRequest) -> Result<(), SysError> {
        let principal = sanitize_principal_id(&req.principal_id)?;
        match write_codex_files() {
            Ok(()) => publish_complete(&principal, true, Some(false), None),
            Err(err) => publish_complete(&principal, false, None, Some(err.to_string())),
        }
    }
}

fn run_install(principal: &str, force: bool) -> Result<bool, SysError> {
    let key = marker_key(principal);
    if !force {
        if let Some(marker) = kv::get_json_opt::<InstallMarker>(&key)? {
            if marker.artifact_version >= ARTIFACT_VERSION {
                return Ok(true);
            }
            publish_status(principal, "reconcile_stale_artifacts")?;
            write_codex_files()?;
            write_marker(&key)?;
            return Ok(true);
        }
    }

    publish_status(principal, "create_dirs")?;
    fs::create_dir_all("home://.codex")?;
    fs::create_dir_all("home://.codex/hooks")?;

    publish_status(principal, "write_config")?;
    write_codex_files()?;

    write_marker(&key)?;
    Ok(false)
}

fn write_marker(key: &str) -> Result<(), SysError> {
    kv::set_json(
        key,
        &InstallMarker {
            version: env!("CARGO_PKG_VERSION").to_string(),
            installed_at: time::now()?
                .duration_since(UNIX_EPOCH)
                .map_err(|err| SysError::ApiError(err.to_string()))?
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            artifact_version: ARTIFACT_VERSION,
        },
    )
}

fn write_codex_files() -> Result<(), SysError> {
    atomic::write_atomic(
        "home://.codex/config.toml",
        br#"# Astrid-managed Codex base config. Principal-specific policy lives in
# sibyl.config.toml so callers can launch with `codex --profile sibyl`.

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
    atomic::write_atomic(
        "home://.codex/sibyl.config.toml",
        br#"approval_policy = "on-request"
sandbox_mode = "workspace-write"
default_permissions = ":workspace"
model_reasoning_summary = "auto"
model_verbosity = "medium"

[features]
hooks = true
"#,
    )?;
    atomic::write_atomic(
        "home://.codex/hooks.json",
        br#"{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "astrid doctor --format hook",
            "timeout": 15,
            "statusMessage": "Checking Astrid"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh -c 'sid=\"${ASTRID_SESSION_ID:-unknown}\"; sid=$(printf \"%s\" \"$sid\" | sed \"s/[^A-Za-z0-9_-]/-/g\"); astrid emit --topic \"sibyl.v1.hook.$sid.stop\" --stdin'",
            "timeout": 15
          }
        ]
      }
    ]
  }
}
"#,
    )?;
    atomic::write_atomic(
        "home://.codex/sibyl.requirements.toml",
        br#"# Staged managed Codex requirements for Sibyl fleets.
# Codex only treats requirements as managed when an admin installs them in a
# managed requirements location. This per-principal copy is documentation plus
# a source body for a future host-side managed-config mount.

allowed_approval_policies = ["on-request"]
allowed_sandbox_modes = ["read-only", "workspace-write"]
default_permissions = ":workspace"

[allowed_permission_profiles]
":read-only" = true
":workspace" = true
"#,
    )
}

fn publish_status(principal: &str, step: &'static str) -> Result<(), SysError> {
    ipc::publish_json(
        STATUS_TOPIC,
        &InstallStatus {
            principal_id: principal.to_string(),
            step,
        },
    )
}

fn publish_complete(
    principal: &str,
    success: bool,
    already_installed: Option<bool>,
    error: Option<String>,
) -> Result<(), SysError> {
    ipc::publish_json(
        COMPLETE_TOPIC,
        &InstallComplete {
            principal_id: principal.to_string(),
            success,
            already_installed,
            error,
            artifact_version: success.then_some(ARTIFACT_VERSION),
        },
    )
}

fn marker_key(principal: &str) -> String {
    format!("{MARKER_PREFIX}.{principal}")
}

fn sanitize_principal_id(id: &str) -> Result<String, SysError> {
    if id.is_empty() {
        return Err(SysError::ApiError("principal_id must not be empty".into()));
    }
    if id == "." || id == ".." {
        return Err(SysError::ApiError("principal_id is reserved".into()));
    }
    if id.len() > 128 {
        return Err(SysError::ApiError(
            "principal_id exceeds 128 characters".into(),
        ));
    }
    for c in id.chars() {
        if !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-') {
            return Err(SysError::ApiError(format!(
                "principal_id contains disallowed character '{c}'"
            )));
        }
    }
    Ok(id.to_string())
}
