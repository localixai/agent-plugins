//! Port of `src/settings/mcp/load-settings.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::view::{PolicyView, create_policy_view};

/// The switches that decide what loads next to, or instead of, the MCP servers a user configured.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one switch per flag"
)]
pub struct McpLoadSettings {
    /// `allowAllClaudeAiMcps`: the connectors Claude Code fetches load alongside `managed-mcp.json`.
    pub allow_all_claude_ai_mcps: bool,
    /// `allowClaudeInChromeWithManagedMcp`, from the device's own managed settings only.
    pub allow_claude_in_chrome_with_managed_mcp: bool,
    /// `disableClaudeAiConnectors` in any file, or `ENABLE_CLAUDEAI_MCP_SERVERS=false`.
    pub disable_claude_ai_connectors: bool,
    /// `strictPluginOnlyCustomization` locks the `mcp` surface.
    pub plugin_only_mcp: bool,
}

/// What the host tells the switches besides the view.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoadFacts {
    /// The host's environment.
    pub env: IndexMap<String, JsValue>,
    /// The managed layers of the device itself: an MDM plist, an HKLM registry key, a system
    /// `managed-settings.json`. A layer has one `managed` scope, so the host says which of its
    /// managed layers these are (spec/judgments.md).
    pub device_managed: Vec<JsValue>,
    /// `strictPluginOnlyCustomization` locks the `mcp` surface.
    pub plugin_only_mcp: bool,
}

/// `allowAllClaudeAiMcps` is "read ... only from admin-controlled policy tiers" and among the keys
/// "read from every admin source"; `disableClaudeAiConnectors` is a `true` "in any settings file"
/// (and the environment switch of the same effect, which the setting cannot undo);
/// `allowClaudeInChromeWithManagedMcp` is "read ... only from those device sources". A `true` from
/// any holder applies (the rows' ladder) (spec/judgments.md for the combination of admin sources).
pub async fn read_mcp_load_settings(
    view: &dyn PolicyView,
    facts: &McpLoadFacts,
) -> McpLoadSettings {
    let device = create_policy_view(&facts.device_managed);
    let on = |value: Option<JsValue>| value == Some(JsValue::Bool(true));
    McpLoadSettings {
        allow_all_claude_ai_mcps: on(view.get("allowAllClaudeAiMcps").await),
        allow_claude_in_chrome_with_managed_mcp: device.value("allowClaudeInChromeWithManagedMcp")
            == JsValue::Bool(true),
        disable_claude_ai_connectors: on(view.get("disableClaudeAiConnectors").await)
            || facts
                .env
                .get("ENABLE_CLAUDEAI_MCP_SERVERS")
                .and_then(JsValue::as_str)
                == Some("false"),
        plugin_only_mcp: facts.plugin_only_mcp,
    }
}

crate::registry::exports! {
    "settings/mcp/load-settings" {
        "readMcpLoadSettings" => async ok |view: Port, facts: McpLoadFacts|
            read_mcp_load_settings(&view, &facts).await;
    }
}
