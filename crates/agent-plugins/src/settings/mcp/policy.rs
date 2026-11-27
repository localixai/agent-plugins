//! Port of `src/settings/mcp/policy.ts`, switched.
//!
//! The MCP keys of the settings policy: the approval of a project's `.mcp.json` servers and the
//! allow and deny lists. The view and the context are the host's (`PolicyView`, a
//! `PolicyContext` holding `isFolderTrusted`), kept as ports and read when a method runs, as the
//! TypeScript's closures read them.

use std::sync::Arc;

use serde::Deserialize;

use crate::js::JsValue;
use crate::js::value::JsObject;
use crate::registry::{Handle, Own, Port};
use crate::settings::layers::PolicyLayer;
use crate::settings::mcp::allow_deny::{McpPolicyEnv, McpServerPolicy, create_mcp_server_policy};
use crate::settings::mcp::approvals::{
    ApprovalSession, McpTrust, ProjectServerApproval, project_server_approval,
};
use crate::settings::mcp::exclusive::{
    McpLoadInput, McpLoadPlan, McpSessionFacts, ServerPolicy, mcp_load_plan,
};
use crate::settings::mcp::load_settings::{McpLoadFacts, McpLoadSettings, read_mcp_load_settings};
use crate::settings::mcp::managed::{DesktopManaged, ManagedServerOptions};
use crate::settings::mcp::managed_file::ManagedMcpFile;
use crate::settings::mcp::managed_servers::{ProvidedServers, read_managed_mcp_servers};
use crate::settings::mcp::toggles::ProjectToggles;

/// What the host adds to the plan: the file it read, the servers of the other scopes and the session.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPlanFacts {
    /// `readManagedMcpFile(...).file`; omit it when no file is deployed or it cannot be read.
    #[serde(default)]
    pub managed_file: Option<ManagedMcpFile>,
    /// The servers of the other scopes.
    pub candidates: Vec<JsValue>,
    /// The session.
    pub session: McpSessionFacts,
    /// The project's `/mcp` toggles.
    #[serde(default)]
    pub toggles: Option<ProjectToggles>,
    /// A Claude Desktop session that supplies its own servers: no `managedMcpServers` entry is read.
    #[serde(default)]
    pub desktop_managed: Option<DesktopManaged>,
}

/// What a host adds to the view: facts no settings file holds.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPolicyExtras {
    /// `strictPluginOnlyCustomization` locks the `mcp` surface (the customization policy knows).
    #[serde(default)]
    pub plugin_only_mcp: Option<bool>,
    /// The managed layers of the device itself (MDM, HKLM, system `managed-settings.json`).
    #[serde(default)]
    pub device_managed: Option<Vec<JsValue>>,
}

/// The MCP keys of the settings policy over one view.
pub struct McpSettingsPolicy {
    view: Port,
    context: Port,
    extras: McpPolicyExtras,
}

impl McpSettingsPolicy {
    /// `context.env`.
    async fn env(&self) -> JsObject {
        self.context
            .member::<Option<JsObject>>("env")
            .await
            .unwrap_or_default()
    }

    /// `view.layers`.
    async fn layers(&self) -> Vec<JsValue> {
        self.view
            .member::<Option<Vec<JsValue>>>("layers")
            .await
            .unwrap_or_default()
    }

    /// The status of each server name of the folder's `.mcp.json`. The folder's trust is the
    /// context's (`isFolderTrusted('project')`); a host that knows more (the configuration home, a
    /// git-tracked `settings.local.json`) passes it in `trust`.
    async fn project_server_approval(
        &self,
        servers: &[String],
        trust: Option<JsObject>,
        session: Option<ApprovalSession>,
    ) -> Vec<ProjectServerApproval> {
        let layers = self.layers().await;
        let trusted: JsValue = self
            .context
            .method("isFolderTrusted", vec![JsValue::from("project")])
            .await;
        let mut given = JsObject::new();
        given.insert("folderTrusted".to_owned(), trusted);
        given.extend(trust.unwrap_or_default());
        let trust = McpTrust::deserialize(JsValue::Object(given)).unwrap_or(McpTrust {
            folder_trusted: false,
            config_home: None,
            local_tracked: None,
        });
        let session = session.unwrap_or(ApprovalSession { prompts: true });
        project_server_approval(servers, &layers, &trust, &session)
    }

    /// `allowedMcpServers`, `deniedMcpServers` and `allowManagedMcpServersOnly` over one server. A
    /// policy entry expands in the context's environment; the server's own `${VAR}` expands in
    /// `live` (default: the same environment).
    async fn server_policy(&self, live: Option<JsObject>) -> McpServerPolicy {
        let startup = self.env().await;
        let live = live.unwrap_or_else(|| startup.clone());
        let env = McpPolicyEnv { startup, live };
        create_mcp_server_policy(&self.view, &env).await
    }

    /// The switches `mcpLoadPlan` reads: the two allow keys, `disableClaudeAiConnectors` and the
    /// `mcp` lock.
    async fn load_settings(&self) -> McpLoadSettings {
        let facts = McpLoadFacts {
            env: self.env().await,
            device_managed: self.extras.device_managed.clone().unwrap_or_default(),
            plugin_only_mcp: self.extras.plugin_only_mcp.unwrap_or(false),
        };
        read_mcp_load_settings(&self.view, &facts).await
    }

    /// `managedMcpServers` of the managed layers, each entry checked (docs/API.md, "MCP settings").
    async fn provided_servers(&self, options: ManagedServerOptions) -> ProvidedServers {
        let layers: Vec<PolicyLayer> = self
            .layers()
            .await
            .into_iter()
            .filter_map(|layer| PolicyLayer::deserialize(layer).ok())
            .collect();
        read_managed_mcp_servers(&layers, &options)
    }

    /// `mcpLoadPlan` over the view: the provided servers, the settings and the lists are the
    /// policy's; the host adds the file it read, the servers of the other scopes and the session.
    async fn load_plan(&self, facts: McpPlanFacts) -> McpLoadPlan {
        let options = ManagedServerOptions {
            desktop_managed: facts.desktop_managed,
        };
        let provided = self.provided_servers(options).await;
        let input = McpLoadInput {
            managed_file: facts.managed_file,
            managed_servers: provided.servers,
            candidates: facts.candidates,
            settings: self.load_settings().await,
            policy: ServerPolicy::Core(Own(Arc::new(self.server_policy(None).await))),
            session: facts.session,
            toggles: facts.toggles,
        };
        mcp_load_plan(&input).await
    }
}

crate::registry::methods! {
    McpSettingsPolicy {
        "projectServerApproval" => sync async ok
            |this, servers: Vec<String>, trust: Option<JsObject>, session: Option<ApprovalSession>|
            this.project_server_approval(&servers, trust, session).await;
        "serverPolicy" => sync async ok |this, live: Option<JsObject>|
            Handle::new(this.server_policy(live).await);
        "loadSettings" => sync async ok |this| this.load_settings().await;
        "providedServers" => sync async ok |this, options: Option<ManagedServerOptions>|
            this.provided_servers(options.unwrap_or_default()).await;
        "loadPlan" => sync async ok |this, facts: McpPlanFacts| this.load_plan(facts).await;
    }
}

/// The MCP keys of the settings policy over `view`, with the host's `context` and `extras`.
#[must_use]
pub fn create_mcp_settings_policy(
    view: Port,
    context: Port,
    extras: McpPolicyExtras,
) -> McpSettingsPolicy {
    McpSettingsPolicy {
        view,
        context,
        extras,
    }
}

crate::registry::exports! {
    "settings/mcp/policy" {
        "createMcpSettingsPolicy" => ok |view: Port, context: Port, extras: Option<McpPolicyExtras>|
            Handle::new(create_mcp_settings_policy(view, context, extras.unwrap_or_default()));
    }
}
