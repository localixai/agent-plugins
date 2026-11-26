//! Port of `src/format/mcpb-manifest.ts`, switched.
//!
//! What an MCPB bundle's own `manifest.json` says (plugins/components.md, "Include a packaged MCPB
//! server": "The server takes its name from the `name` in the bundle's manifest" and "A bundle's own
//! manifest can declare settings the server needs from the user in a `user_config` block"). The format
//! itself lives at github.com/modelcontextprotocol/mcpb, not in the Claude Code docs, so every rule here is
//! a judgment (spec/judgments.md). Pure data: extracting or downloading the archive is the host's.
//!
//! A `UserConfigOption` is the shape of `src/format/manifest-schema.ts`, whose module is not in the
//! core yet: what this module makes of a setting is `McpbOption`, and what it only reads
//! (`required`, `default`) of one it is given is `OptionState`.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue, Pattern, in_property_order, is_truthy, property_order};

/// The types a bundle setting has.
pub const MCPB_OPTION_TYPES: [&str; 5] = ["string", "number", "boolean", "directory", "file"];

/// A bundle's setting in the plugin option shape (`UserConfigOption`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpbOption {
    /// One of `MCPB_OPTION_TYPES`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// What the dialog calls it.
    pub title: String,
    /// What it is for.
    pub description: String,
    /// The bundle's own mark.
    pub required: bool,
    /// Whether the value is kept out of sight.
    pub sensitive: bool,
    /// A text, number or Boolean the setting starts with.
    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub fallback: Option<JsValue>,
}

/// What a bundle's manifest says (`McpbManifest`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpbManifest {
    /// The server's name: the manifest's `name`.
    pub name: String,
    /// The `server.mcp_config` (or the one an `entry_point` stands for) for this platform; `${...}` unresolved.
    pub server: JsObject,
    /// The `user_config` block by key, in the plugin option shape.
    pub options: IndexMap<String, McpbOption>,
}

/// The manifest, or why there is none (`McpbRead`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum McpbRead {
    /// The name, server and settings.
    Manifest {
        /// What was read.
        manifest: McpbManifest,
    },
    /// Why nothing was read.
    Problem {
        /// The sentence.
        problem: String,
    },
}

/// What `server.entry_point` stands for when the manifest has no `mcp_config`.
fn entry_command(kind: &str, entry: &str) -> JsObject {
    let script = || JsValue::from(vec![JsValue::from(format!("${{__dirname}}/{entry}"))]);
    match kind {
        "node" => JsObject::from([
            ("command".to_owned(), "node".into()),
            ("args".to_owned(), script()),
        ]),
        "python" => JsObject::from([
            ("command".to_owned(), "python3".into()),
            ("args".to_owned(), script()),
        ]),
        _ => binary_command(entry),
    }
}

fn binary_command(entry: &str) -> JsObject {
    JsObject::from([(
        "command".to_owned(),
        format!("${{__dirname}}/{entry}").into(),
    )])
}

fn record(value: Option<&JsValue>) -> Option<&JsObject> {
    match value {
        Some(JsValue::Object(members)) => Some(members),
        Some(_) | None => None,
    }
}

/// The server config of a manifest's `server` block, with this platform's `platform_overrides` over it.
fn server_of(server: &JsObject, platform: &str) -> Option<JsObject> {
    let entry = server.get("entry_point").and_then(JsValue::as_str);
    let kind = server
        .get("type")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let stand_in = entry.map(|each| entry_command(kind, each));
    let config = match record(server.get("mcp_config")) {
        Some(declared) => declared.clone(),
        None => stand_in?,
    };
    let overrides = record(config.get("platform_overrides"));
    let own = overrides.and_then(|each| record(each.get(platform)));
    let mut merged = config.clone();
    for (key, value) in own.map(property_order).unwrap_or_default() {
        merged.insert(key.clone(), value.clone());
    }
    merged.shift_remove("platform_overrides");
    Some(in_property_order(merged))
}

fn option_type(raw: Option<&JsValue>) -> &'static str {
    MCPB_OPTION_TYPES
        .into_iter()
        .find(|kind| raw.and_then(JsValue::as_str) == Some(*kind))
        .unwrap_or("string")
}

fn option_of(key: &str, raw: &JsObject) -> McpbOption {
    let text = |member: &str| raw.get(member).and_then(JsValue::as_str).map(str::to_owned);
    let fallback = raw.get("default").filter(|each| {
        matches!(
            each,
            JsValue::String(_) | JsValue::Number(_) | JsValue::Bool(_)
        )
    });
    McpbOption {
        kind: option_type(raw.get("type")),
        title: text("title").unwrap_or_else(|| key.to_owned()),
        description: text("description").unwrap_or_default(),
        required: raw.get("required") == Some(&JsValue::Bool(true)),
        sensitive: raw.get("sensitive") == Some(&JsValue::Bool(true)),
        fallback: fallback.cloned(),
    }
}

/// A bundle's `user_config` block in the plugin option shape. Judgments (spec/judgments.md): an entry
/// that is not an object is skipped, a `type` the kit does not know is `string`, a setting with no
/// `title` is titled by its key, and a `default` that is not text, a number or a Boolean is none.
/// Reading a block already read gives it back.
#[must_use]
pub fn read_mcpb_options(declared: &JsValue) -> IndexMap<String, McpbOption> {
    let JsValue::Object(members) = declared else {
        return IndexMap::new();
    };
    property_order(members)
        .into_iter()
        .filter_map(|(key, raw)| match raw {
            JsValue::Object(setting) => Some((key.clone(), option_of(key, setting))),
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_)
            | JsValue::Array(_) => None,
        })
        .collect()
}

/// The name, server and settings of a bundle manifest for `platform` (`darwin`, `linux`, `win32`).
/// Judgment: a `type` other than `node` or `python` with an `entry_point` runs that file as a binary.
#[must_use]
pub fn read_mcpb_manifest(manifest: &JsValue, platform: &str) -> McpbRead {
    let name = record(Some(manifest)).and_then(|each| each.get("name")?.as_str());
    let (Some(members), Some(name)) = (record(Some(manifest)), name) else {
        return McpbRead::Problem {
            problem: "the bundle manifest has no name".to_owned(),
        };
    };
    let server = record(members.get("server")).and_then(|each| server_of(each, platform));
    let Some(server) = server else {
        return McpbRead::Problem {
            problem: "the bundle manifest declares no server".to_owned(),
        };
    };
    let options = read_mcpb_options(members.get("user_config").unwrap_or(&JsValue::Undefined));
    McpbRead::Manifest {
        manifest: McpbManifest {
            name: name.to_owned(),
            server,
            options,
        },
    }
}

/// The key a bundled server's setting has among the plugin's settings: `<server>.<key>`.
#[must_use]
pub fn bundle_setting_key(server: &str, key: &str) -> String {
    format!("{server}.{key}")
}

/// What `requiredBundleSettings` reads of a setting.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct OptionState {
    /// The bundle's own mark.
    #[serde(default)]
    pub required: JsValue,
    /// What it starts with.
    #[serde(default)]
    pub default: JsValue,
}

/// What `requiredBundleSettings` reads of a manifest: `Pick<McpbManifest, 'name' | 'options'>`.
#[derive(Clone, Debug, Deserialize)]
pub struct RequiredInput {
    /// The server's name.
    pub name: String,
    /// The settings by key.
    pub options: IndexMap<String, OptionState>,
}

/// The settings of a bundle the server cannot start without (plugins/components.md: "A bundled server
/// with a required setting that has no saved value doesn't start"), as `<server>.<key>`. Judgment: a
/// required setting with a default of its own is not one.
#[must_use]
pub fn required_bundle_settings(manifest: &RequiredInput) -> Vec<String> {
    manifest
        .options
        .iter()
        .filter(|(_, option)| is_truthy(&option.required) && option.default.is_undefined())
        .map(|(key, _)| bundle_setting_key(&manifest.name, key))
        .collect()
}

/// The settings among `required` that `saved` has no value for: the one rule both the host's dialog and
/// the kit's load-time check use. Judgment (spec/judgments.md): an empty text is no value, as a
/// required plugin option's dialog "doesn't accept an empty value" (plugins/manifest-reference.md).
#[must_use]
pub fn missing_bundle_settings(required: &[String], saved: &JsObject) -> Vec<String> {
    required
        .iter()
        .filter(|key| match saved.get(key.as_str()) {
            None | Some(JsValue::Undefined) => true,
            Some(value) => value.as_str() == Some(""),
        })
        .cloned()
        .collect()
}

fn map_strings(value: &JsValue, change: &dyn Fn(&str) -> String) -> JsValue {
    match value {
        JsValue::String(text) => JsValue::String(change(text)),
        JsValue::Array(items) => {
            JsValue::Array(items.iter().map(|each| map_strings(each, change)).collect())
        }
        JsValue::Object(members) => JsValue::Object(
            property_order(members)
                .into_iter()
                .map(|(key, each)| (key.clone(), map_strings(each, change)))
                .collect(),
        ),
        JsValue::Undefined | JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) => value.clone(),
    }
}

/// `/\$\{([^}]+)\}/g`.
static VARIABLE: Pattern = LazyLock::new(|| Regex::new(r"\$\{([^}]+)\}").ok());

/// Where the bundle was extracted and what the variables stand for.
#[derive(Clone, Debug, Deserialize)]
pub struct McpbVariables {
    /// Where the bundle was extracted: `${__dirname}`.
    pub directory: String,
    /// The home folder.
    pub home: String,
    /// `/`, or `\` on Windows: `${/}` and `${pathSeparator}`.
    pub separator: String,
}

/// What `bundleServerOf` reads of a manifest: `McpbManifest['server']`.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerInput {
    /// The server config, `${...}` unresolved.
    pub server: JsObject,
}

/// The manifest's `${...}` variables in every string of the server: `${__dirname}`, `${HOME}`, `${/}` and
/// `${pathSeparator}`. `${user_config.<key>}` stays as written: the kit fills it from the bundle's own
/// saved settings when it resolves the server, in its one substitution pass; any other variable is left
/// as written.
#[must_use]
pub fn bundle_server_of(manifest: &ServerInput, variables: &McpbVariables) -> JsObject {
    let text = |source: &str| -> String {
        let Some(pattern) = VARIABLE.as_ref() else {
            return source.to_owned();
        };
        pattern
            .replace_all(source, |found: &Captures<'_>| {
                let name = found.get(1).map_or("", |each| each.as_str());
                match name {
                    "__dirname" => variables.directory.clone(),
                    "HOME" => variables.home.clone(),
                    "/" | "pathSeparator" => variables.separator.clone(),
                    _ => found
                        .get(0)
                        .map_or_else(String::new, |each| each.as_str().to_owned()),
                }
            })
            .into_owned()
    };
    property_order(&manifest.server)
        .into_iter()
        .map(|(key, each)| (key.clone(), map_strings(each, &text)))
        .collect()
}

crate::registry::exports! {
    "format/mcpb-manifest" {
        "readMcpbOptions" => ok |declared: JsValue| read_mcpb_options(&declared);
        "readMcpbManifest" => ok |manifest: JsValue, platform: String| read_mcpb_manifest(&manifest, &platform);
        "bundleSettingKey" => ok |server: String, key: String| bundle_setting_key(&server, &key);
        "requiredBundleSettings" => ok |manifest: RequiredInput| required_bundle_settings(&manifest);
        "missingBundleSettings" => ok |required: Vec<String>, saved: JsObject| missing_bundle_settings(&required, &saved);
        "bundleServerOf" => ok |manifest: ServerInput, variables: McpbVariables| bundle_server_of(&manifest, &variables);
    }
}
