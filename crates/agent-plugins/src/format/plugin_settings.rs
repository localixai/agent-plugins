//! Port of `src/format/plugin-settings.ts`, switched.
//!
//! A plugin's default settings (plugins/components.md, "Default settings"; plugins/manifest-reference.md,
//! "`settings`"): the `settings.json` at the plugin root or the inline `settings` manifest key. "Two
//! keys take effect, `agent` and `subagentStatusLine`, and every other key is dropped." The types are
//! the official `Settings` ones; applying the values (the lowest settings layer) is the host's.
//!
//! `PluginSettings` (`Pick<Settings, 'agent' | 'subagentStatusLine'>`) is held as the object it
//! is, its keys in the order the file wrote them, as the TypeScript builds it.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, fields};

/// The shape the SDK gives `subagentStatusLine` (settings-reference.md: `type` is `"command"`, plus a `command` string).
static SUBAGENT_STATUS_LINE: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! { "type" => zod::literal("command"), "command" => zod::string() })
});

static AGENT: LazyLock<Schema> = LazyLock::new(zod::string);

/// One strategy per applied key: the value it keeps, or `undefined` when it is not valid.
fn applied(name: &str) -> Option<fn(&JsValue) -> Option<JsValue>> {
    match name {
        "agent" => Some(|value| AGENT.safe_parse(value).ok()),
        "subagentStatusLine" => Some(|value| SUBAGENT_STATUS_LINE.safe_parse(value).ok()),
        _ => None,
    }
}

/// What reading one settings object gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginSettingsRead {
    /// Only the keys that take effect and hold a valid value.
    pub settings: JsObject,
    /// What was dropped, and why.
    pub issues: Vec<PluginIssue>,
}

/// True when a read holds at least one applied key: `settings.json` then wins over the manifest.
#[must_use]
pub fn has_applied_settings(read: &PluginSettingsRead) -> bool {
    !read.settings.is_empty()
}

/// Reads one settings object. Judgment (spec/judgments.md): a dropped key and an invalid value for an
/// applied key are warnings (the docs say "dropped" and name no message), and a value that is not an
/// object is a warning naming `origin`: a plugin's defaults never fail the plugin (spec §12.6 item 1,
/// J12.10).
#[must_use]
pub fn read_plugin_settings(raw: &JsValue, origin: &str) -> PluginSettingsRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return PluginSettingsRead {
            settings: JsObject::new(),
            issues: vec![warning(
                origin,
                &format!("{origin} must be a JSON object of settings"),
            )],
        };
    };
    let mut issues: Vec<PluginIssue> = Vec::new();
    let mut kept = JsObject::new();
    for (name, value) in property_order(members) {
        let Some(apply) = applied(name) else {
            issues.push(warning(
                &format!("{origin}.{name}"),
                &format!(
                    "Setting \"{name}\" in {origin} is dropped: only \"agent\" and \"subagentStatusLine\" take effect from a plugin"
                ),
            ));
            continue;
        };
        match apply(value) {
            None | Some(JsValue::Undefined) => issues.push(warning(
                &format!("{origin}.{name}"),
                &format!("Setting \"{name}\" in {origin} is dropped: invalid value"),
            )),
            Some(accepted) => {
                kept.insert(name.clone(), accepted);
            }
        }
    }
    PluginSettingsRead {
        settings: kept,
        issues,
    }
}

crate::registry::exports! {
    "format/plugin-settings" {
        "hasAppliedSettings" => ok |read: PluginSettingsRead| has_applied_settings(&read);
        "readPluginSettings" => ok |raw: JsValue, origin: String| read_plugin_settings(&raw, &origin);
    }
}
