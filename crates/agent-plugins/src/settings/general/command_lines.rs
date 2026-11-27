//! Port of `src/settings/general/command-lines.ts`, switched.
//!
//! settings-reference.md, "Status line and file suggestion gates": commands follow hook policy.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue, property_order};
use crate::registry::Port;
use crate::settings::layers::PolicyScope;
use crate::settings::repair::support::{RepairAt, Repaired, kept, must_be, note};
use crate::settings::view::PolicyView;
use crate::zod::{self, Schema};

/// The settings keys whose value is a command line.
pub const COMMAND_LINE_KEYS: [&str; 3] = ["statusLine", "fileSuggestion", "subagentStatusLine"];

/// The check of each optional field.
fn optional(key: &str) -> Option<fn(&JsValue) -> bool> {
    match key {
        "padding" => Some(|value| value.as_f64().is_some_and(f64::is_finite)),
        "refreshInterval" => Some(|value| {
            value
                .as_f64()
                .is_some_and(|number| number >= 1.0 && number.is_finite())
        }),
        "hideVimModeIndicator" => Some(JsValue::is_boolean),
        _ => None,
    }
}

/// Invalid optional fields are dropped individually; the required command must be readable.
static COMMAND: LazyLock<Schema> = LazyLock::new(|| zod::literal("command"));

/// The repair of a command line.
#[must_use]
pub fn command_line(value: &JsValue, at: &RepairAt) -> Repaired {
    let JsValue::Object(members) = value else {
        return must_be(at, "an object with type \"command\" and a command string");
    };
    let r#type = members.get("type").cloned().unwrap_or_default();
    if COMMAND.safe_parse(&r#type).is_err()
        || !members.get("command").is_some_and(JsValue::is_string)
    {
        return must_be(at, "an object with type \"command\" and a command string");
    }
    let mut issues: Vec<PluginIssue> = Vec::new();
    let mut kept_fields = JsObject::new();
    for (key, field) in property_order(members) {
        let valid = optional(key);
        if valid.is_none_or(|valid| valid(field)) {
            kept_fields.insert(key.clone(), field.clone());
            continue;
        }
        issues.push(note(
            at,
            &format!(".{key}"),
            &format!("\"{}.{key}\" is invalid; the field is dropped", at.key),
        ));
    }
    kept(JsValue::Object(kept_fields), issues)
}

/// Why a command line does not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Skipped {
    /// The policy turned it off.
    Off,
    /// Only managed settings may set it, and none does.
    Narrowed,
}

/// The command that runs and the scope that set it.
#[derive(Clone, Debug, Serialize)]
pub struct ScopedRun {
    /// The command line.
    pub value: JsValue,
    /// The scope that set it.
    pub scope: PolicyScope,
}

/// The selected command line, or why there is none.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRead {
    /// What runs; `undefined` when nothing does.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub runs: Option<ScopedRun>,
    /// Why nothing runs, when the policy says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_because: Option<Skipped>,
}

/// The members of a `HookPolicy` read here.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookPolicyRead {
    /// Each command key the policy turns off.
    #[serde(default)]
    pub commands_disabled: IndexMap<String, bool>,
    /// Only managed settings may set a command.
    #[serde(default)]
    pub command_settings_narrowed_to_managed: bool,
}

/// A `HookPolicy` as it crosses: one that holds its session gates (functions) is the host's, whose
/// members are read one by one; one that holds none is data (docs/RUST.md, "Effects").
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HookPolicy {
    /// The policy holds functions.
    Host(Port),
    /// The policy is data.
    Data(HookPolicyRead),
}

impl HookPolicy {
    /// The members read here.
    pub async fn read(self) -> HookPolicyRead {
        match self {
            Self::Data(read) => read,
            Self::Host(port) => HookPolicyRead {
                commands_disabled: port
                    .member::<Option<IndexMap<String, bool>>>("commandsDisabled")
                    .await
                    .unwrap_or_default(),
                command_settings_narrowed_to_managed: port
                    .member::<Option<bool>>("commandSettingsNarrowedToManaged")
                    .await
                    .unwrap_or_default(),
            },
        }
    }
}

/// The selected command, never an execution: the host supplies the hook policy's session gates.
pub async fn read_command_line(
    view: &impl PolicyView,
    key: &str,
    policy: HookPolicy,
) -> CommandRead {
    let policy = policy.read().await;
    if policy
        .commands_disabled
        .get(key)
        .copied()
        .unwrap_or_default()
    {
        return CommandRead {
            runs: None,
            skipped_because: Some(Skipped::Off),
        };
    }
    let narrowed = policy.command_settings_narrowed_to_managed;
    let holder = view
        .each_from_highest(key)
        .await
        .into_iter()
        .find(|entry| !narrowed || entry.scope == PolicyScope::Managed);
    let Some(holder) = holder else {
        return CommandRead {
            runs: None,
            skipped_because: narrowed.then_some(Skipped::Narrowed),
        };
    };
    CommandRead {
        runs: Some(ScopedRun {
            value: holder.value,
            scope: holder.scope,
        }),
        skipped_because: None,
    }
}

/// The subagent status line that runs: a settings source's (with its scope) or a plugin's default.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum SubagentLineRead {
    /// A settings source's, or none.
    Settings(CommandRead),
    /// A plugin's default.
    Plugin {
        /// The command line and its plugin.
        runs: PluginRun,
    },
}

/// A plugin's default command line.
#[derive(Clone, Debug, Serialize)]
pub struct PluginRun {
    /// The command line.
    pub value: JsValue,
    /// The plugin, as the host passed it.
    pub plugin: JsValue,
}

/// `subagentStatusLine` with the plugins' defaults below every settings source (plugins/components.md,
/// "Default settings"), under the same gates as `statusLine`. statusline.md, "Subagent status lines":
/// "unlike hooks, plugin values don't run under `allowManagedHooksOnly` even when the plugin is
/// force-enabled", so a narrowed policy never runs a plugin's value.
pub async fn read_subagent_status_line(
    view: &impl PolicyView,
    policy: HookPolicy,
    defaults: &JsValue,
) -> SubagentLineRead {
    let read = read_command_line(view, "subagentStatusLine", policy).await;
    if read.runs.is_some() || read.skipped_because.is_some() {
        return SubagentLineRead::Settings(read);
    }
    let of = |member: &str| {
        defaults
            .get(member)
            .and_then(|block| block.get("subagentStatusLine"))
            .filter(|value| !value.is_undefined())
            .cloned()
    };
    match (of("settings"), of("owners")) {
        (Some(value), Some(plugin)) => SubagentLineRead::Plugin {
            runs: PluginRun { value, plugin },
        },
        _ => SubagentLineRead::Settings(read),
    }
}

crate::registry::exports! {
    "settings/general/command-lines" {
        "COMMAND_LINE_KEYS" => ok || COMMAND_LINE_KEYS;
        "readCommandLine" => async ok |view: Port, key: String, policy: HookPolicy|
            read_command_line(&view, &key, policy).await;
        "readSubagentStatusLine" => async ok |view: Port, policy: HookPolicy, defaults: JsValue|
            read_subagent_status_line(&view, policy, &defaults).await;
    }
}
