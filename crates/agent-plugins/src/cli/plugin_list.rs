//! Port of `src/cli/plugin-list.ts`, switched.
//!
//! `claude plugin list` (plugins/cli-reference.md, "plugin list" and "JSON output"): the groups of
//! the text listing, the `--json` array with the fields the page names, `--available` and
//! `--data-size`. Pure: the host reads the records, the settings and the data directories and passes
//! them in. The layout of a plugin's text lines is the kit's (spec/judgments.md); the group headings,
//! the empty line and every JSON field are the docs'.

use indexmap::IndexMap;
use serde::Deserialize;

use super::result::Rendered;
use crate::js::{JsObject, JsValue, json_stringify, number_to_string, to_fixed};

/// One installation, as the list shows it. Optional fields "appear only when they apply".
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedPlugin {
    /// `name@marketplace`, `name@inline`, `name@skills-dir` or `name@synced`.
    pub id: String,
    /// The computed version, or the manifest's, or `unknown`.
    pub version: String,
    /// `user`, `project`, `local`, `managed`, `session` or `synced`.
    pub scope: String,
    /// Whether it is enabled.
    pub enabled: bool,
    /// The directory it loads from; absent for a plugin loaded in place, which has `readFromFolder`.
    #[serde(default)]
    pub install_path: Option<String>,
    /// The folder a plugin loaded in place reads from.
    #[serde(default)]
    pub read_from_folder: Option<String>,
    /// Its version in that folder.
    #[serde(default)]
    pub folder_version: Option<String>,
    /// When it was installed.
    #[serde(default)]
    pub installed_at: Option<String>,
    /// When it was last updated.
    #[serde(default)]
    pub last_updated: Option<String>,
    /// The project it is installed for.
    #[serde(default)]
    pub project_path: Option<String>,
    /// Its MCP servers.
    #[serde(default)]
    pub mcp_servers: Option<JsObject>,
    /// Its errors.
    #[serde(default)]
    pub errors: Option<Vec<String>>,
    /// Its notes.
    #[serde(default)]
    pub notes: Option<Vec<String>>,
    /// The errors' details.
    #[serde(default)]
    pub error_details: Option<Vec<JsObject>>,
    /// The notes' details.
    #[serde(default)]
    pub note_details: Option<Vec<JsObject>>,
    /// Only for a plugin that loaded and declares `userConfig`.
    #[serde(default)]
    pub has_user_config: Option<bool>,
    /// Marketplace installs only: the project's shared settings turn it on.
    #[serde(default)]
    pub project_enabled: Option<bool>,
}

/// A plugin a marketplace offers that is not installed.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailablePlugin {
    /// Its id.
    pub plugin_id: String,
    /// Its name.
    pub name: String,
    /// The marketplace's name.
    pub marketplace_name: String,
    /// A string or a source object.
    pub source: JsValue,
    /// Its description.
    #[serde(default)]
    pub description: Option<String>,
    /// Its version.
    #[serde(default)]
    pub version: Option<String>,
    /// How many times it was installed.
    #[serde(default)]
    pub install_count: Option<f64>,
}

/// What measuring a saved data directory found; absent for a missing or empty one:
/// `{ bytes }`, or the text `unreadable`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataSize {
    /// The size.
    Measured {
        /// The size in bytes.
        bytes: f64,
    },
    /// `unreadable`.
    Unreadable(String),
}

/// The listing's request.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    /// The installations.
    pub installed: Vec<ListedPlugin>,
    /// The plugins marketplaces offer.
    #[serde(default)]
    pub available: Option<Vec<AvailablePlugin>>,
}

/// `--data-size [plugin]`: `true` for every installed plugin, or one `name@marketplace`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum DataSizeSelector {
    /// `true`.
    Every(bool),
    /// One plugin's id.
    One(String),
}

/// The flags of the listing.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOptions {
    /// `--json`.
    pub json: bool,
    /// `--available`: "has no effect without `--json`".
    #[serde(default)]
    pub available: Option<bool>,
    /// `--data-size [plugin]`: "has no effect without `--json`".
    #[serde(default)]
    pub data_size: Option<DataSizeSelector>,
    /// What the host measured, by id.
    #[serde(default)]
    pub sizes: Option<IndexMap<String, Option<DataSize>>>,
}

/// How the page's four groups are named and which ids belong to them.
const GROUPS: [(Option<&str>, &str); 4] = [
    (None, "Installed plugins:"),
    (
        Some("inline"),
        "Session-only plugins (--plugin-dir / --plugin-url):",
    ),
    (
        Some("skills-dir"),
        "Skills-directory plugins (.claude/skills/*):",
    ),
    (Some("synced"), "Synced from claude.ai"),
];

const NOTHING: &str = "No plugins installed. Use `claude plugin install` to install a plugin.";

fn suffix_of(id: &str) -> &str {
    id.rfind('@')
        .map_or(id, |at| id.get(at + 1..).unwrap_or(id))
}

fn is_other_suffix(suffix: &str) -> bool {
    GROUPS.iter().any(|group| group.0 == Some(suffix))
}

fn in_group(plugin: &ListedPlugin, suffix: Option<&str>) -> bool {
    match suffix {
        None => !is_other_suffix(suffix_of(&plugin.id)),
        Some(suffix) => suffix_of(&plugin.id) == suffix,
    }
}

fn is_session_only(plugin: &ListedPlugin) -> bool {
    suffix_of(&plugin.id) == "inline"
}

/// J17.9: a session-only plugin shows its path instead of a scope, and `loaded` (oracle 2.1.325).
fn where_line(plugin: &ListedPlugin) -> String {
    if is_session_only(plugin) {
        let path = plugin
            .install_path
            .as_deref()
            .or(plugin.read_from_folder.as_deref())
            .unwrap_or("");
        format!("    Path: {path}")
    } else {
        format!("    Scope: {}", plugin.scope)
    }
}

fn status_of(plugin: &ListedPlugin) -> &'static str {
    if !plugin.enabled {
        return "✘ disabled";
    }
    if is_session_only(plugin) {
        "✔ loaded"
    } else {
        "✔ enabled"
    }
}

fn lines(plugin: &ListedPlugin) -> Vec<String> {
    let mut lines = vec![
        format!("  ❯ {}", plugin.id),
        format!("    Version: {}", plugin.version),
        where_line(plugin),
        format!("    Status: {}", status_of(plugin)),
    ];
    lines.extend(
        plugin
            .errors
            .iter()
            .flatten()
            .map(|message| format!("    Error: {message}")),
    );
    lines.extend(
        plugin
            .notes
            .iter()
            .flatten()
            .map(|message| format!("    Note: {message}")),
    );
    lines
}

/// The text listing, grouped by how each plugin loads; nothing in any group is one line.
#[must_use]
pub fn list_text(installed: &[ListedPlugin]) -> Vec<String> {
    let mut groups = Vec::new();
    for (suffix, heading) in GROUPS {
        let members: Vec<&ListedPlugin> = installed
            .iter()
            .filter(|plugin| in_group(plugin, suffix))
            .collect();
        if !members.is_empty() {
            groups.push(heading.to_owned());
            groups.extend(members.into_iter().flat_map(lines));
        }
    }
    if groups.is_empty() {
        vec![NOTHING.to_owned()]
    } else {
        groups
    }
}

const KILO: f64 = 1024.0;

/// `human` of `dataDirSize`: the byte count in the largest binary unit that keeps it above 1.
fn human_bytes(bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes;
    let mut unit = 0;
    while value >= KILO && unit < units.len() - 1 {
        value /= KILO;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", number_to_string(bytes))
    } else {
        format!(
            "{} {}",
            to_fixed(value, 1),
            units.get(unit).copied().unwrap_or("")
        )
    }
}

/// Whether a plugin keeps a saved data directory: "Marketplace installs only".
fn is_marketplace_install(plugin: &ListedPlugin) -> bool {
    !is_other_suffix(suffix_of(&plugin.id))
}

fn size_fields(plugin: &ListedPlugin, options: &ListOptions) -> JsObject {
    let wanted = match &options.data_size {
        Some(DataSizeSelector::Every(every)) => *every,
        Some(DataSizeSelector::One(id)) => *id == plugin.id,
        None => false,
    };
    let found = options
        .sizes
        .as_ref()
        .and_then(|sizes| sizes.get(&plugin.id))
        .and_then(Option::as_ref);
    let mut fields = JsObject::new();
    let Some(found) = found else {
        return fields;
    };
    if !wanted || !is_marketplace_install(plugin) {
        return fields;
    }
    match found {
        DataSize::Unreadable(text) if text == "unreadable" => {
            fields.insert("dataDirUnreadable".to_owned(), true.into());
        }
        DataSize::Measured { bytes } if *bytes > 0.0 => {
            fields.insert(
                "dataDirSize".to_owned(),
                JsValue::from([
                    ("bytes", JsValue::Number(*bytes)),
                    ("human", human_bytes(*bytes).into()),
                ]),
            );
        }
        DataSize::Unreadable(_) | DataSize::Measured { .. } => {}
    }
    fields
}

/// The fields of an object that are set; an empty list of errors or notes does not "apply" either.
fn present(fields: Vec<(&str, Option<JsValue>)>) -> JsObject {
    let mut kept = JsObject::new();
    for (key, value) in fields {
        let Some(value) = value else { continue };
        let empty = matches!(&value, JsValue::Array(items) if items.is_empty());
        if !empty {
            kept.insert(key.to_owned(), value);
        }
    }
    kept
}

fn text(value: Option<&String>) -> Option<JsValue> {
    value.map(|text| JsValue::String(text.clone()))
}

fn strings(values: Option<&Vec<String>>) -> Option<JsValue> {
    values.map(|items| JsValue::Array(items.iter().cloned().map(JsValue::String).collect()))
}

fn objects(values: Option<&Vec<JsObject>>) -> Option<JsValue> {
    values.map(|items| JsValue::Array(items.iter().cloned().map(JsValue::Object).collect()))
}

/// One installed plugin as a JSON object, only the fields that apply.
#[must_use]
pub fn installed_json(plugin: &ListedPlugin, options: &ListOptions) -> JsObject {
    let mut fields = present(vec![
        ("id", Some(plugin.id.clone().into())),
        ("version", Some(plugin.version.clone().into())),
        ("scope", Some(plugin.scope.clone().into())),
        ("enabled", Some(plugin.enabled.into())),
        (
            "installPath",
            if plugin.read_from_folder.is_none() {
                text(plugin.install_path.as_ref())
            } else {
                None
            },
        ),
        ("readFromFolder", text(plugin.read_from_folder.as_ref())),
        ("folderVersion", text(plugin.folder_version.as_ref())),
        ("installedAt", text(plugin.installed_at.as_ref())),
        ("lastUpdated", text(plugin.last_updated.as_ref())),
        ("projectPath", text(plugin.project_path.as_ref())),
        (
            "mcpServers",
            plugin.mcp_servers.clone().map(JsValue::Object),
        ),
        ("errors", strings(plugin.errors.as_ref())),
        ("notes", strings(plugin.notes.as_ref())),
        ("errorDetails", objects(plugin.error_details.as_ref())),
        ("noteDetails", objects(plugin.note_details.as_ref())),
        ("hasUserConfig", plugin.has_user_config.map(JsValue::Bool)),
        ("projectEnabled", plugin.project_enabled.map(JsValue::Bool)),
    ]);
    fields.extend(size_fields(plugin, options));
    fields
}

/// The `available` entry.
#[must_use]
pub fn available_json(plugin: &AvailablePlugin) -> JsObject {
    present(vec![
        ("pluginId", Some(plugin.plugin_id.clone().into())),
        ("name", Some(plugin.name.clone().into())),
        (
            "marketplaceName",
            Some(plugin.marketplace_name.clone().into()),
        ),
        ("source", Some(plugin.source.clone())),
        ("description", text(plugin.description.as_ref())),
        ("version", text(plugin.version.as_ref())),
        ("installCount", plugin.install_count.map(JsValue::Number)),
    ])
}

const NOT_INSTALLED: &str = "--data-size names a plugin that is not installed";

/// What `claude plugin list` prints. `--json` is an array with one object per installation, or with
/// `--available` one object holding `installed` and `available`. A `--data-size` plugin with no install
/// record prints its message and exits `1` "instead of printing the list".
#[must_use]
pub fn plan_plugin_list(request: &ListRequest, options: &ListOptions) -> Rendered {
    if let (Some(DataSizeSelector::One(named)), true) = (&options.data_size, options.json)
        && !request.installed.iter().any(|plugin| plugin.id == *named)
    {
        return Rendered {
            stdout: Vec::new(),
            stderr: vec![NOT_INSTALLED.to_owned()],
            exit_code: 1,
        };
    }
    if !options.json {
        return Rendered {
            stdout: list_text(&request.installed),
            stderr: Vec::new(),
            exit_code: 0,
        };
    }
    let installed = JsValue::Array(
        request
            .installed
            .iter()
            .map(|plugin| JsValue::Object(installed_json(plugin, options)))
            .collect(),
    );
    let printed = if options.available == Some(true) {
        let available = request.available.as_deref().unwrap_or_default();
        JsValue::from([
            ("installed", installed),
            (
                "available",
                JsValue::Array(
                    available
                        .iter()
                        .map(|plugin| JsValue::Object(available_json(plugin)))
                        .collect(),
                ),
            ),
        ])
    } else {
        installed
    };
    Rendered {
        stdout: vec![json_stringify(&printed).unwrap_or_default()],
        stderr: Vec::new(),
        exit_code: 0,
    }
}

crate::registry::exports! {
    "cli/plugin-list" {
        "listText" => ok |installed: Vec<ListedPlugin>| list_text(&installed);
        "installedJson" => ok |plugin: ListedPlugin, options: Option<ListOptions>| installed_json(&plugin, &options.unwrap_or(ListOptions { json: true, available: None, data_size: None, sizes: None }));
        "availableJson" => ok |plugin: AvailablePlugin| available_json(&plugin);
        "planPluginList" => ok |request: ListRequest, options: ListOptions| plan_plugin_list(&request, &options);
    }
}
