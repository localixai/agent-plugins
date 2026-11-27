//! Port of `src/settings/mcp/approvals.ts`, switched.
//!
//! The approvals of a project's `.mcp.json` servers (mcp.md, "Project server approvals and
//! workspace trust"), and whether the session can show the approval prompt.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::settings::layers::{PolicyScope, Ranked};
use crate::settings::view::create_policy_view;

/// What the host knows about the folder (mcp.md, "Project server approvals and workspace trust").
/// The kit runs no git and reads no trust dialog result: the host answers these.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTrust {
    /// The user accepted the workspace trust dialog for this folder.
    pub folder_trusted: bool,
    /// The folder is "your own configuration home: your home directory, or a directory whose
    /// `.claude` you've set as `CLAUDE_CONFIG_DIR`".
    #[serde(default)]
    pub config_home: Option<bool>,
    /// git tracks `.claude/settings.local.json`; the docs exempt only an "untracked" one
    /// (judgment).
    #[serde(default)]
    pub local_tracked: Option<bool>,
}

/// Two of the statuses `claude mcp list` and `claude mcp get` print without connecting (mcp.md,
/// "Server status"): `pending approval` shows in both commands, `rejected` only in `claude mcp
/// get`. `✘ Blocked by enterprise managed policy` is a third such status; a policy decision is not
/// an approval decision, so it is out of scope here (spec/judgments.md).
pub const PROJECT_SERVER_STATUSES: [&str; 3] = ["approved", "pending approval", "rejected"];

/// One server's status, and what decided it (absent while it is pending).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectServerApproval {
    /// The server's name.
    pub name: String,
    /// One of `PROJECT_SERVER_STATUSES`.
    pub status: &'static str,
    /// `disabledMcpjsonServers`, `enableAllProjectMcpServers`, `enabledMcpjsonServers` or
    /// `session`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<&'static str>,
}

/// Whether the session can show the approval prompt (`canPromptForProjectServers`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ApprovalSession {
    /// It can.
    pub prompts: bool,
}

/// The files whose approvals count. User settings, managed settings and `--settings` count in an
/// untrusted folder; the shared project file counts once the folder is trusted; an untracked
/// `.claude/settings.local.json` counts in a trusted folder or the configuration home. The parent
/// settings of an embedding host and `--add-dir` directories approve nothing (judgment,
/// spec/judgments.md): `--add-dir` settings carry two keys only (permissions.md) and parent
/// settings keep "only the host's values that restrict" (managed-settings.md); the settings key
/// registry admits no MCP approval key from either, so their `disabledMcpjsonServers` does not
/// reject either.
fn honours(scope: PolicyScope, trust: &McpTrust) -> bool {
    match scope {
        PolicyScope::Managed | PolicyScope::Flag | PolicyScope::User => true,
        PolicyScope::Project => trust.folder_trusted,
        PolicyScope::Local => {
            trust.folder_trusted
                || (trust.config_home == Some(true) && trust.local_tracked != Some(true))
        }
        PolicyScope::Parent | PolicyScope::AddDir => false,
    }
}

/// `enableAllProjectMcpServers`: "In a trusted folder, a `false` in a higher-precedence file
/// overrides a `true` in a lower one; in a folder you haven't trusted, a `true` in any honored
/// file is enough."
fn approves_all(honoured: &[JsValue], trust: &McpTrust) -> bool {
    let holders = create_policy_view(honoured).all("enableAllProjectMcpServers");
    if trust.folder_trusted {
        holders
            .first()
            .is_some_and(|holder| holder.value == JsValue::Bool(true))
    } else {
        holders
            .iter()
            .any(|holder| crate::js::is_truthy(&holder.value))
    }
}

fn names_of(value: &JsValue) -> Vec<String> {
    let JsValue::Array(names) = value else {
        return Vec::new();
    };
    names
        .iter()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect()
}

fn approved(name: &str, via: &'static str) -> ProjectServerApproval {
    ProjectServerApproval {
        name: name.to_owned(),
        status: "approved",
        via: Some(via),
    }
}

/// The status of each server of a project's `.mcp.json` (`servers` are its names, in the order to
/// report). A server is pending until `enableAllProjectMcpServers` or `enabledMcpjsonServers`
/// approves it; a `disabledMcpjsonServers` entry in any file, honoured or not, rejects it and takes
/// precedence. A session that cannot prompt (`claude -p`, Agent SDK, cloud) loads a pending server
/// without asking.
#[must_use]
pub fn project_server_approval(
    servers: &[String],
    layers: &[JsValue],
    trust: &McpTrust,
    session: &ApprovalSession,
) -> Vec<ProjectServerApproval> {
    let honoured: Vec<JsValue> = layers
        .iter()
        .filter(|layer| layer.scope_of().is_some_and(|scope| honours(scope, trust)))
        .cloned()
        .collect();
    let named = names_of(&create_policy_view(&honoured).value("enabledMcpjsonServers"));
    let rejected = names_of(&create_policy_view(layers).value("disabledMcpjsonServers"));
    let all = approves_all(&honoured, trust);
    servers
        .iter()
        .map(|name| {
            if rejected.contains(name) {
                return ProjectServerApproval {
                    name: name.clone(),
                    status: "rejected",
                    via: Some("disabledMcpjsonServers"),
                };
            }
            if all {
                return approved(name, "enableAllProjectMcpServers");
            }
            if named.contains(name) {
                return approved(name, "enabledMcpjsonServers");
            }
            if session.prompts {
                ProjectServerApproval {
                    name: name.clone(),
                    status: "pending approval",
                    via: None,
                }
            } else {
                approved(name, "session")
            }
        })
        .collect()
}

/// Where `skipDangerousModePermissionPrompt` counts for this: "your user settings or ... managed
/// settings".
const fn skips_prompt_from(scope: PolicyScope) -> bool {
    matches!(scope, PolicyScope::Managed | PolicyScope::User)
}

/// The session the prompt would show in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSession {
    /// An interactive session; `claude -p`, Agent SDK and cloud sessions "can't show that prompt".
    pub interactive: bool,
    /// The session runs in `bypassPermissions` mode.
    pub bypass_permissions: bool,
}

/// Whether the session shows the approval prompt (mcp.md, "Project scope"): interactive sessions
/// do, except one in `bypassPermissions` mode with `skipDangerousModePermissionPrompt` set in user
/// or managed settings.
#[must_use]
pub fn can_prompt_for_project_servers(session: &PromptSession, layers: &[JsValue]) -> bool {
    let skips = layers.iter().any(|layer| {
        layer.scope_of().is_some_and(skips_prompt_from)
            && layer
                .get("settings")
                .and_then(|settings| settings.get("skipDangerousModePermissionPrompt"))
                == Some(&JsValue::Bool(true))
    });
    session.interactive && !(session.bypass_permissions && skips)
}

crate::registry::exports! {
    "settings/mcp/approvals" {
        "PROJECT_SERVER_STATUSES" => ok || PROJECT_SERVER_STATUSES;
        "projectServerApproval" => ok |servers: Vec<String>, layers: Vec<JsValue>, trust: McpTrust, session: Option<ApprovalSession>|
            project_server_approval(&servers, &layers, &trust, &session.unwrap_or(ApprovalSession { prompts: true }));
        "canPromptForProjectServers" => ok |session: PromptSession, layers: Vec<JsValue>| can_prompt_for_project_servers(&session, &layers);
    }
}
