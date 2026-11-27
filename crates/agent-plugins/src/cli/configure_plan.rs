//! Port of `src/cli/configure-plan.ts`, switched.
//!
//! `claude plugin configure <plugin>` as a plan (plugins/cli-reference.md, "plugin configure"): the
//! plugin's `userConfig` options and which are set, or the values piped in on stdin validated and
//! saved. Only the full `name@marketplace` id is accepted ("`configure` doesn't accept a bare
//! `name`"). A saved value is never printed except, with `--json`, a value of an option that isn't
//! sensitive ("never the text of a sensitive one"). Pure over the loaded plugins; the host reads
//! stdin and stores what the plan names: "Non-sensitive values are saved under `pluginConfigs` ...
//! Sensitive values go to the platform's secure credential store" (manifest-reference.md, "Where
//! values are stored"). What the docs leave open is in spec/judgments.md.

use serde::{Deserialize, Serialize};

use super::result::{CommandOutput, Extras, ResultFields, command_failed, command_succeeded};
use super::user_config_input::{ConfigProblem, check_given, unset_keys, with_values};
use crate::foundation::json_value::to_json_value;
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, property_order, string_of};
use crate::loading::resolve::user_config::{ResolvedUserConfig, user_config_value};

/// What `configure` was given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureArguments {
    /// The plugin's full `name@marketplace` id.
    pub plugin: String,
    /// The text read from stdin; present exactly when `--values-stdin` is passed.
    #[serde(default)]
    pub values_stdin: Option<String>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
}

/// A plugin that loaded, with its declared options and what is saved for them.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigurablePlugin {
    /// Its id.
    pub id: String,
    /// Its options and their saved values.
    pub config: ResolvedUserConfig,
}

/// What the plan reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureFacts {
    /// The plugins that loaded.
    pub loaded: Vec<ConfigurablePlugin>,
    /// The host could read the saved values back after `--values-stdin`; `false` leaves `unconfigured`
    /// out of the `--json` object ("when they could be read back, the `unconfigured` ones"). Default `true`.
    #[serde(default)]
    pub read_back: Option<bool>,
}

/// What the host stores, split by where it goes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedValues {
    /// `pluginConfigs` in the user's settings.
    pub settings: JsObject,
    /// The platform's secure credential store.
    pub secrets: JsObject,
}

/// What `configure` prints and saves.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfigurePlan {
    /// What it prints.
    pub output: CommandOutput,
    /// Set when values are to be stored; a refused save sets nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save: Option<SavedValues>,
}

/// The line of a save that went through.
pub const SAVED_MESSAGE: &str = "Configuration saved. Restart Claude Code to apply it.";
/// The start of the line of a save that was refused.
pub const SAVE_FAILED: &str = "Failed to save configuration:";

const fn nothing(output: CommandOutput) -> ConfigurePlan {
    ConfigurePlan { output, save: None }
}

fn fields_of<const N: usize>(members: [(&str, JsValue); N]) -> ResultFields {
    members
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn not_installed(plugin: &str) -> ConfigurePlan {
    nothing(command_failed(
        "configure",
        &format!("No installed plugin has the id \"{plugin}\"."),
        Extras {
            failure_code: Some("not_installed".to_owned()),
            fields: None,
        },
    ))
}

/// `option?.<name> === true`.
fn flag_of(option: Option<&JsValue>, name: &str) -> bool {
    option.and_then(|option| option.get(name)) == Some(&JsValue::Bool(true))
}

/// `required` or `optional`, then `sensitive`, then `set` or `not set` ("up to three labels").
fn labels_of(config: &ResolvedUserConfig, key: &str, unset: &[String]) -> Vec<&'static str> {
    let option = config.declared.get(key);
    let mut labels = vec![if flag_of(option, "required") {
        "required"
    } else {
        "optional"
    }];
    if flag_of(option, "sensitive") {
        labels.push("sensitive");
    }
    labels.push(if unset.iter().any(|one| one == key) {
        "not set"
    } else {
        "set"
    });
    labels
}

/// `declared[key]?.title ?? key`.
fn title_of(config: &ResolvedUserConfig, key: &str) -> String {
    match config
        .declared
        .get(key)
        .and_then(|option| option.get("title"))
    {
        None | Some(JsValue::Undefined | JsValue::Null) => key.to_owned(),
        Some(title) => string_of(title),
    }
}

/// One line per option; never a saved value.
fn listing(plugin: &ConfigurablePlugin) -> String {
    let keys: Vec<&String> = property_order(&plugin.config.declared)
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    if keys.is_empty() {
        return format!("{} declares no userConfig options", plugin.id);
    }
    let unset = unset_keys(&plugin.config);
    let lines = keys.into_iter().map(|key| {
        format!(
            "{key} ({}): {}",
            labels_of(&plugin.config, key, &unset).join(", "),
            title_of(&plugin.config, key)
        )
    });
    std::iter::once(format!("Options of {}:", plugin.id))
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The value an option starts a dialog with: saved, else the default; never a sensitive option's.
fn inputs_of(config: &ResolvedUserConfig) -> JsObject {
    let mut inputs = JsObject::new();
    for (key, option) in property_order(&config.declared) {
        let value = user_config_value(config, key);
        if option.get("sensitive") != Some(&JsValue::Bool(true))
            && let Some(value) = value.filter(|value| *value != JsValue::Undefined)
        {
            inputs.insert(key.clone(), value);
        }
    }
    inputs
}

fn configured_of(config: &ResolvedUserConfig) -> Vec<String> {
    let unset = unset_keys(config);
    property_order(&config.declared)
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| !unset.contains(key))
        .cloned()
        .collect()
}

fn choices_of(config: &ResolvedUserConfig) -> JsObject {
    let mut choices = JsObject::new();
    for (key, option) in property_order(&config.declared) {
        match option.get("options") {
            None | Some(JsValue::Undefined) => {}
            Some(options) => {
                choices.insert(key.clone(), options.clone());
            }
        }
    }
    choices
}

fn texts(list: Vec<String>) -> JsValue {
    JsValue::Array(list.into_iter().map(JsValue::String).collect())
}

/// The object `--json` prints without `--values-stdin`.
fn describe_plugin(plugin: &ConfigurablePlugin) -> ConfigurePlan {
    let config = &plugin.config;
    let schema = to_json_value(&JsValue::Object(config.declared.clone()))
        .unwrap_or_else(|| JsValue::Object(JsObject::new()));
    let fields = fields_of([
        ("pluginId", plugin.id.as_str().into()),
        ("schema", schema),
        ("choices", JsValue::Object(choices_of(config))),
        ("inputs", JsValue::Object(inputs_of(config))),
        ("configured", texts(configured_of(config))),
        ("unconfigured", texts(unset_keys(config))),
    ]);
    nothing(command_succeeded(
        "configure",
        &listing(plugin),
        Extras {
            failure_code: None,
            fields: Some(fields),
        },
    ))
}

/// The `--values-stdin` text as an object of single-line strings, or the problems with it.
fn read_given(text: &str) -> Result<JsObject, Vec<ConfigProblem>> {
    let Ok(parsed) = json_parse(text) else {
        return Err(vec![whole("stdin is not valid JSON")]);
    };
    let JsValue::Object(parsed) = parsed else {
        return Err(vec![whole(
            "stdin must be a JSON object that maps option keys to string values",
        )]);
    };
    let mut given = JsObject::new();
    let mut problems = Vec::new();
    for (key, value) in property_order(&parsed) {
        if let JsValue::String(text) = value
            && !text.contains(['\r', '\n'])
        {
            given.insert(key.clone(), value.clone());
        } else {
            problems.push(ConfigProblem {
                key: key.clone(),
                undeclared: false,
                message: format!("the value of \"{key}\" must be a single-line string"),
            });
        }
    }
    if problems.is_empty() {
        Ok(given)
    } else {
        Err(problems)
    }
}

/// A problem that is not one option's: it carries no option key.
fn whole(message: &str) -> ConfigProblem {
    ConfigProblem {
        key: String::new(),
        undeclared: false,
        message: message.to_owned(),
    }
}

fn refusal(problems: &[ConfigProblem]) -> ConfigurePlan {
    let message = problems
        .iter()
        .map(|problem| problem.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let option = match problems {
        [only] if !only.key.is_empty() => Some(only.key.clone()),
        _ => None,
    };
    let mut refused = fields_of([("message", message.as_str().into())]);
    if let Some(option) = option {
        refused.insert("option".to_owned(), JsValue::String(option));
    }
    nothing(command_failed(
        "configure",
        &format!("{SAVE_FAILED} {message}"),
        Extras {
            failure_code: Some("invalid_values".to_owned()),
            fields: Some(fields_of([("refused", JsValue::Object(refused))])),
        },
    ))
}

fn split(plugin: &ConfigurablePlugin, accepted: &JsObject) -> SavedValues {
    let mut saved = SavedValues::default();
    for (key, value) in property_order(accepted) {
        let sensitive = flag_of(plugin.config.declared.get(key), "sensitive");
        let place = if sensitive {
            &mut saved.secrets
        } else {
            &mut saved.settings
        };
        place.insert(key.clone(), value.clone());
    }
    saved
}

fn save_values(plugin: &ConfigurablePlugin, text: &str, read_back: bool) -> ConfigurePlan {
    let given = match read_given(text) {
        Ok(given) => given,
        Err(problems) => return refusal(&problems),
    };
    let checked = check_given(&plugin.config.declared, &given);
    if !checked.problems.is_empty() {
        return refusal(&checked.problems);
    }
    let after = with_values(plugin.config.clone(), checked.accepted.clone());
    let saved: Vec<String> = property_order(&checked.accepted)
        .into_iter()
        .map(|(key, _)| key.clone())
        .collect();
    let mut fields = fields_of([
        ("pluginId", plugin.id.as_str().into()),
        ("saved", texts(saved)),
    ]);
    if read_back {
        fields.insert("unconfigured".to_owned(), texts(unset_keys(&after)));
    }
    ConfigurePlan {
        output: command_succeeded(
            "configure",
            SAVED_MESSAGE,
            Extras {
                failure_code: None,
                fields: Some(fields),
            },
        ),
        save: Some(split(plugin, &checked.accepted)),
    }
}

/// What `claude plugin configure` prints and saves.
#[must_use]
pub fn plan_configure(args: &ConfigureArguments, facts: &ConfigureFacts) -> ConfigurePlan {
    let Some(plugin) = facts.loaded.iter().find(|one| one.id == args.plugin) else {
        return not_installed(&args.plugin);
    };
    match &args.values_stdin {
        None => describe_plugin(plugin),
        Some(text) => save_values(plugin, text, facts.read_back != Some(false)),
    }
}

crate::registry::exports! {
    "cli/configure-plan" {
        "SAVED_MESSAGE" => ok || SAVED_MESSAGE;
        "SAVE_FAILED" => ok || SAVE_FAILED;
        "planConfigure" => ok |args: ConfigureArguments, facts: ConfigureFacts| plan_configure(&args, &facts);
    }
}
