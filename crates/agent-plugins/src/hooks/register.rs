//! Port of `src/hooks/register.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::protocol::hook_events::HOOK_EVENTS;

/// Where a hooks file was declared: what every entry made from it carries.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookDeclaration {
    /// Prefix of the entry ids; two declarations of one registry never share one.
    pub id_prefix: String,
    /// The `HookSource` of the declaration, kept as the host gave it.
    pub source: JsValue,
    /// The full `name@marketplace` id of the plugin, for a plugin's hooks.
    #[serde(default)]
    pub plugin_id: Option<String>,
}

/// One group of a hooks file: a matcher and the handlers it applies to.
#[derive(Clone, Debug, Deserialize)]
pub struct HookGroup {
    /// The matcher of the group.
    #[serde(default)]
    pub matcher: Option<String>,
    /// The handlers, each the official object.
    pub hooks: Vec<JsValue>,
}

/// A handler a registry holds, with the matcher of its group and where it was declared.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredHandler {
    /// `<prefix>:<event>:<group>:<handler>`.
    pub id: String,
    /// The event it fires on.
    pub event: &'static str,
    /// The matcher of its group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The official handler object.
    pub handler: JsValue,
    /// Where it was declared.
    pub source: JsValue,
    /// The full plugin id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

/// A hooks file, in the shape of the `hooks` key of settings (hooks.md, "Configuration"; a plugin's
/// `hooks/hooks.json` wraps it the same way), as the handlers a registry holds: one entry per
/// handler, in file order, with the matcher of its group and where it was declared. The handler
/// stays the official object. A key that names no event of the official union registers nothing:
/// no input is ever built for it.
#[must_use]
pub fn register_hooks(
    hooks: &IndexMap<String, Vec<HookGroup>>,
    declaration: &HookDeclaration,
) -> Vec<RegisteredHandler> {
    let mut entries = Vec::new();
    for (name, groups) in hooks {
        let Some(event) = HOOK_EVENTS.iter().find(|known| *known == name) else {
            continue;
        };
        for (group_at, group) in groups.iter().enumerate() {
            for (handler_at, handler) in group.hooks.iter().enumerate() {
                entries.push(RegisteredHandler {
                    id: format!("{}:{event}:{group_at}:{handler_at}", declaration.id_prefix),
                    event,
                    matcher: group.matcher.clone(),
                    handler: handler.clone(),
                    source: declaration.source.clone(),
                    plugin_id: declaration.plugin_id.clone(),
                });
            }
        }
    }
    entries
}

crate::registry::exports! {
    "hooks/register" {
        "registerHooks" => ok |hooks: IndexMap<String, Vec<HookGroup>>, declaration: HookDeclaration| register_hooks(&hooks, &declaration);
    }
}
