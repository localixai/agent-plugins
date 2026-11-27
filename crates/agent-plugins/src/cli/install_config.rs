//! Port of `src/cli/install-config.ts`, switched.
//!
//! `plugin install --config <key=value>` (plugins/cli-reference.md, "plugin install"; plugins/
//! troubleshooting.md, "The `userConfig` dialog never appears"; plugins/components.md, "Include a
//! packaged MCPB server"): each flag sets "a `userConfig` option the plugin's manifest declares";
//! "A key written `<server>.<key>` sets a setting that a bundled MCP server declares in its own
//! `user_config` instead". The install "never prompts": a key the manifest doesn't declare still
//! installs the plugin and prints `⚠ Installed, but --config not applied: ...`; options left unset
//! print `N userConfig options not yet set — run /plugin configure <id> in Claude Code, or pass
//! --config KEY=VALUE.` with `(M required)` after `not yet set` when one is required. Values are
//! checked like `plugin configure`'s (`checkGiven`). A value that fails the check refuses the
//! install (judgment): the docs name the refusal for `configure` only. A key given twice keeps its
//! last value.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::configure_plan::SavedValues;
use super::result::{CommandOutput, usage_error};
use super::user_config_input::{ConfigProblem, check_given, unset_keys, with_values};
use crate::js::{JsObject, JsValue, in_key_order, in_property_order, property_order};
use crate::loading::resolve::user_config::ResolvedUserConfig;

/// The `userConfig` of the plugin and of each bundled MCP server (by server name).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    /// The plugin's own options.
    pub declared: JsObject,
    /// Each bundled MCP server's options, by server name.
    #[serde(default)]
    pub bundled_servers: Option<IndexMap<String, JsObject>>,
    /// Bundled MCP servers whose bundle the manifest references by URL: "isn't read at install time, so
    /// its keys aren't listed". Their keys are not known keys, and the message says to configure them in `/plugin`.
    #[serde(default)]
    pub unread_servers: Option<Vec<String>>,
}

/// The `--config` flags as `key` → `value`, or the usage error of a flag with no `=`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParsedConfigFlags {
    /// `{ given }`.
    Given {
        /// The values by key.
        given: JsObject,
    },
    /// A flag with no `=`.
    Usage(CommandOutput),
}

/// The `--config` flags as `key` → `value`; a flag with no `=` is a usage error.
#[must_use]
pub fn parse_config_flags(flags: &[String]) -> ParsedConfigFlags {
    let mut given = JsObject::new();
    for flag in flags {
        let Some((key, value)) = flag.split_once('=').filter(|(key, _)| !key.is_empty()) else {
            return ParsedConfigFlags::Usage(usage_error(&format!(
                "--config expects key=value, got \"{flag}\""
            )));
        };
        given.insert(key.to_owned(), JsValue::from(value));
    }
    ParsedConfigFlags::Given {
        given: in_property_order(given),
    }
}

/// The bundled servers of the options, in the order `Object.entries` gives them.
fn bundled_of(options: &InstallOptions) -> IndexMap<String, JsObject> {
    in_key_order(options.bundled_servers.clone().unwrap_or_default())
}

/// Every key a `--config` can name: the plugin's options and `<server>.<key>` for each bundled server's.
fn known_keys(options: &InstallOptions) -> JsObject {
    let mut known = options.declared.clone();
    for (server, declared) in bundled_of(options) {
        for (key, option) in property_order(&declared) {
            known.insert(format!("{server}.{key}"), option.clone());
        }
    }
    in_property_order(known)
}

/// What the `--config` values do to an install.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOutcome {
    /// Failing values: the install is refused.
    pub refused: Vec<ConfigProblem>,
    /// Keys that name no option: the install goes ahead and says they were not applied.
    pub not_applied: Vec<ConfigProblem>,
    /// What to store: the plugin's own options.
    pub plugin: SavedValues,
    /// And each bundled server's by server name.
    pub servers: IndexMap<String, JsObject>,
    /// The plugin's own options still unset after the install.
    pub unset: Vec<String>,
    /// How many of them are required.
    pub unset_required: usize,
}

fn split_saved(accepted: &JsObject, declared: &JsObject) -> SavedValues {
    let mut saved = SavedValues::default();
    for (key, value) in property_order(accepted) {
        let sensitive = declared.get(key).and_then(|option| option.get("sensitive"))
            == Some(&JsValue::Bool(true));
        let place = if sensitive {
            &mut saved.secrets
        } else {
            &mut saved.settings
        };
        place.insert(key.clone(), value.clone());
    }
    saved
}

fn servers_of(accepted: &JsObject, options: &InstallOptions) -> IndexMap<String, JsObject> {
    let mut servers: IndexMap<String, JsObject> = IndexMap::new();
    for server in bundled_of(options).keys() {
        let prefix = format!("{server}.");
        for (key, value) in property_order(accepted) {
            if let Some(rest) = key.strip_prefix(&prefix) {
                servers
                    .entry(server.clone())
                    .or_default()
                    .insert(rest.to_owned(), value.clone());
            }
        }
    }
    servers
        .into_iter()
        .map(|(server, values)| (server, in_property_order(values)))
        .collect()
}

/// What the `--config` values do to an install.
#[must_use]
pub fn apply_config(given: &JsObject, options: &InstallOptions) -> ConfigOutcome {
    let checked = check_given(&known_keys(options), given);
    let own: JsObject = property_order(&checked.accepted)
        .into_iter()
        .filter(|(key, _)| options.declared.contains_key(key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let after = with_values(
        ResolvedUserConfig {
            declared: options.declared.clone(),
            values: JsObject::new(),
        },
        own.clone(),
    );
    let unset = unset_keys(&after);
    let (not_applied, refused) = checked
        .problems
        .into_iter()
        .partition(|problem| problem.undeclared);
    let unset_required = unset
        .iter()
        .filter(|key| {
            options
                .declared
                .get(key.as_str())
                .and_then(|option| option.get("required"))
                == Some(&JsValue::Bool(true))
        })
        .count();
    ConfigOutcome {
        refused,
        not_applied,
        plugin: if own.is_empty() {
            SavedValues::default()
        } else {
            split_saved(&own, &options.declared)
        },
        servers: servers_of(&checked.accepted, options),
        unset,
        unset_required,
    }
}

/// `N userConfig options not yet set (M required) — run /plugin configure <id> in Claude Code, or pass --config KEY=VALUE.`
#[must_use]
pub fn not_yet_set_line(id: &str, outcome: &ConfigOutcome) -> Option<String> {
    if outcome.unset.is_empty() {
        return None;
    }
    let required = if outcome.unset_required > 0 {
        format!(" ({} required)", outcome.unset_required)
    } else {
        String::new()
    };
    Some(format!(
        "{} userConfig options not yet set{required} — run /plugin configure {id} in Claude Code, or pass --config KEY=VALUE.",
        outcome.unset.len()
    ))
}

/// One `⚠ Installed, but --config not applied` line per key that names no option, each followed by the
/// keys that do exist. A bundle the manifest references by URL adds `Configure "<server>" in /plugin` (judgment: the
/// docs say only that "the message says to configure it in `/plugin`").
#[must_use]
pub fn not_applied_lines(outcome: &ConfigOutcome, options: &InstallOptions) -> Vec<String> {
    let unread = options.unread_servers.clone().unwrap_or_default();
    let bundled = !bundled_of(options).is_empty() || !unread.is_empty();
    let place = if bundled {
        "this plugin's userConfig or by its bundled MCP servers"
    } else {
        "this plugin's userConfig"
    };
    let keys: Vec<String> = known_keys(options).into_keys().collect();
    let known = if keys.is_empty() {
        "none".to_owned()
    } else {
        keys.join(", ")
    };
    let configure = unread
        .iter()
        .map(|server| {
            format!(". Configure \"{server}\" in /plugin: its bundle is not read at install time")
        })
        .collect::<Vec<_>>()
        .concat();
    outcome
        .not_applied
        .iter()
        .map(|problem| {
            format!(
                "⚠ Installed, but --config not applied: --config key \"{}\" isn't declared in {place}. Declared keys: {known}{configure}",
                problem.key
            )
        })
        .collect()
}

crate::registry::exports! {
    "cli/install-config" {
        "applyConfig" => ok |given: JsObject, options: InstallOptions| apply_config(&given, &options);
        "notYetSetLine" => ok |id: String, outcome: ConfigOutcome| not_yet_set_line(&id, &outcome);
    }
}
