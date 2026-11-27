//! Port of `src/settings/customization/policy.ts`, switched.
//!
//! `strictPluginOnlyCustomization` as one object over a view (docs/API.md, "Settings"): a host's
//! discovery of skills, agents, hooks and MCP servers asks `loads(surface, origin)` per candidate.
//! The host holds each question as a function of the core's.

use std::collections::HashSet;
use std::sync::Arc;

use serde::Serialize;

use super::preapproval::{PreapprovalSkill, allowed_tools_preapproved};
use super::sources::loads;
use super::surfaces::read_surface_locks;
use crate::js::JsValue;
use crate::registry::{Function, Port, Set};
use crate::settings::managed::parent::{ParentSettingsContext, effective_view};
use crate::settings::marketplaces::gate::{MarketplaceGate, create_marketplace_gate};
use crate::settings::view::PolicyView;

/// What `claude mcp add` says while the `mcp` surface is locked: the message errors.md quotes for
/// "Cannot add MCP server when managed settings allow only plugin servers".
const MCP_ADD_REFUSAL: &str = "Cannot add MCP server: your organization's managed settings allow only MCP servers that plugins provide. Install a plugin that provides this server, or ask your administrator to make it available.";

/// The customization policy as the host holds it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomizationPolicy {
    /// The kinds of customization the lock covers.
    pub locked: Set<String>,
    /// Names in the list that no kind has: they lock nothing, and the status note counts them.
    pub unknown_surface_names: f64,
    /// Whether a customization of `surface` from `origin` loads.
    pub loads: Function,
    /// Whether skills-directory plugins load, and which key stops them when they do not.
    pub skills_dir_plugins: Function,
    /// Whether the skill's `allowed-tools` pre-approval holds.
    pub skill_allowed_tools_preapproved: Function,
    /// The refusal of `claude mcp add` while MCP servers are locked to plugins; `undefined`
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_add_refusal: Option<&'static str>,
}

impl CustomizationPolicy {
    /// Whether the lock covers `surface`.
    #[must_use]
    pub fn has(&self, surface: &str) -> bool {
        self.locked.members.iter().any(|each| each == surface)
    }
}

struct Loads(Arc<[String]>);

crate::registry::methods! {
    Loads {
        "call" => ok |this, surface: String, origin: JsValue| loads(&this.0, &surface, &origin);
    }
}

struct SkillsDirPlugins(MarketplaceGate);

crate::registry::methods! {
    SkillsDirPlugins {
        "call" => ok |this| this.0.skills_directory_plugins();
    }
}

struct Preapproved(HashSet<String>);

crate::registry::methods! {
    Preapproved {
        "call" => ok |this, skill: PreapprovalSkill| allowed_tools_preapproved(&this.0, &skill);
    }
}

/// The customization policy over a view the parent rules were already applied to
/// (`resolvePolicy`).
pub async fn customization_over(effective: &dyn PolicyView) -> CustomizationPolicy {
    let read = read_surface_locks(
        &effective
            .get("strictPluginOnlyCustomization")
            .await
            .unwrap_or_default(),
    );
    let locked: Vec<String> = read
        .locked
        .members
        .iter()
        .map(|&each| each.to_owned())
        .collect();
    let gate = create_marketplace_gate(effective).await;
    let mcp = locked.iter().any(|each| each == "mcp");
    CustomizationPolicy {
        loads: Function::new(Loads(locked.clone().into())),
        skills_dir_plugins: Function::new(SkillsDirPlugins(gate)),
        skill_allowed_tools_preapproved: Function::new(Preapproved(
            locked.iter().cloned().collect(),
        )),
        mcp_add_refusal: mcp.then_some(MCP_ADD_REFUSAL),
        locked: Set { members: locked },
        unknown_surface_names: read.unknown_names,
    }
}

/// The key passes the parent filter (claude-apps-gateway.md), but a parent is still ignored where
/// an administrator deploys managed settings and does not let an embedding host add policy.
pub async fn create_customization_policy(
    view: Arc<dyn PolicyView>,
    context: &ParentSettingsContext,
) -> CustomizationPolicy {
    customization_over(&*effective_view(view, context).await).await
}

crate::registry::exports! {
    "settings/customization/policy" {
        "customizationOver" => async ok |effective: Port| customization_over(&effective).await;
        "createCustomizationPolicy" => async ok |view: Port, context: ParentSettingsContext|
            create_customization_policy(Arc::new(view), &context).await;
    }
}
