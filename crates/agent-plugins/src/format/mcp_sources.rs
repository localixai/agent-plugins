//! Port of `src/format/mcp-sources.ts`, switched.
//!
//! Where a plugin's MCP servers come from (plugins/manifest-reference.md, "mcpServers"): "Claude Code
//! loads `.mcp.json` at the plugin root first, then each declared shape in order. A server name
//! declared later replaces an earlier one." The four shapes of the table are the `McpSourceKind`
//! members; each has one strategy in a registry typed over the union.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error};
use super::locate::locate;
use super::mcp_bundles::{McpLoadContext, load_bundle};
use super::mcp_config::{McpBundleConfig, McpConfigRead, read_mcp_config, rejects_manifest};
use super::path_escape::LinkRoots;
use super::path_text::{PathPlatform, root_relative};
use super::read_json::{JsonRead, parse_json};
use crate::js::{JsObject, JsValue, in_key_order, in_property_order, json_stringify, string_of};
use crate::ports::Signal;
use crate::ports::files::TextFileReader;
use crate::ports::mcp_bundle::McpBundleSource;
use crate::protocol::is_record::is_record;
use crate::registry::{CorePromise, Function, Port};

/// The four shapes of a manifest `mcpServers` value.
pub const MCP_SOURCE_KINDS: [&str; 4] = ["json-file", "bundle-path", "bundle-url", "inline-map"];

/// One declared source, ready to load (its `kind` and what `load` reads).
#[derive(Clone, Debug, PartialEq)]
pub enum McpSource {
    /// `json-file`.
    JsonFile(String),
    /// `bundle-path`.
    BundlePath(String),
    /// `bundle-url`.
    BundleUrl(String),
    /// `inline-map`.
    InlineMap(JsObject),
}

const FIELD: &str = "mcpServers";
const ROOT_FILE: &str = ".mcp.json";

fn none(issues: Vec<PluginIssue>) -> McpConfigRead {
    McpConfigRead {
        servers: JsObject::new(),
        bundles: None,
        issues,
        rejects_plugin: None,
    }
}

fn failed(path: &str, message: &str) -> McpConfigRead {
    none(vec![error(path, message)])
}

/// A `.json` file holding an `mcpServers` map, wrapper or bare (components.md, "MCP servers").
/// Containment is `locate`'s, as for every other component path; the root `.mcp.json` is one of
/// the plugin's own files, so a link of it that leads outside is reported by its full path.
async fn load_json_file(path: &str, context: &McpLoadContext, own: bool) -> McpConfigRead {
    let located = locate(&context.located(), FIELD, path, Some(own)).await;
    if let Some(issue) = located.issue {
        return none(vec![issue]);
    }
    let text = if located.stat.is_some_and(|stat| stat.is_file) {
        let file = root_relative(path, context.platform);
        context
            .files
            .read_text(&file, context.signal.as_ref())
            .await
    } else {
        None
    };
    // A missing file contributes no servers and the loader reports nothing; a validator does (§8.1 item 5).
    let Some(text) = text else {
        return none(Vec::new());
    };
    match parse_json(&text) {
        JsonRead::Problem { problem, .. } => failed(
            path,
            &format!("{FIELD} {path} is not valid JSON: {problem}"),
        ),
        JsonRead::Value { value } if is_record(&value) => read_mcp_config(&value, path),
        JsonRead::Value { .. } => failed(
            path,
            &format!("{FIELD} {path} must hold a JSON object of servers"),
        ),
    }
}

/// A bundle in the plugin folder: contained as any other component path, then opened.
async fn load_bundle_path(value: &str, context: &McpLoadContext) -> McpConfigRead {
    let located = locate(&context.located(), FIELD, value, None).await;
    if let Some(issue) = located.issue {
        return none(vec![issue]);
    }
    let source = McpBundleSource::Path {
        path: root_relative(value, context.platform),
    };
    load_bundle(value, &source, context).await
}

impl McpSource {
    /// Its kind, one of `MCP_SOURCE_KINDS`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::JsonFile(_) => "json-file",
            Self::BundlePath(_) => "bundle-path",
            Self::BundleUrl(_) => "bundle-url",
            Self::InlineMap(_) => "inline-map",
        }
    }

    /// Reads what it declares.
    pub async fn load(&self, context: &McpLoadContext) -> McpConfigRead {
        match self {
            Self::BundleUrl(value) => {
                let source = McpBundleSource::Url { url: value.clone() };
                load_bundle(value, &source, context).await
            }
            Self::JsonFile(value) => load_json_file(value, context, false).await,
            Self::BundlePath(value) => load_bundle_path(value, context).await,
            Self::InlineMap(value) => {
                let mut read = read_mcp_config(&JsValue::Object(value.clone()), FIELD);
                if value.values().any(rejects_manifest) {
                    read.rejects_plugin = Some(true);
                }
                read
            }
        }
    }
}

/// `/^https?:\/\//i`.
fn is_web_url(value: &str) -> bool {
    let lower = value.get(..8).unwrap_or(value).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// One strategy per kind: `None` when the value is not of that kind.
fn strategies(kind: &str, value: &JsValue) -> Option<McpSource> {
    let text = value.as_str();
    match kind {
        "bundle-url" => text
            .filter(|t| is_web_url(t))
            .map(|t| McpSource::BundleUrl(t.to_owned())),
        "json-file" => text
            .filter(|t| t.ends_with(".json"))
            .map(|t| McpSource::JsonFile(t.to_owned())),
        // Any other string is a bundle path: a wrong extension then fails as "must end in .mcpb or .dxt".
        "bundle-path" => text.map(|t| McpSource::BundlePath(t.to_owned())),
        _ => value
            .as_object()
            .filter(|_| is_record(value))
            .map(|members| McpSource::InlineMap(members.clone())),
    }
}

/// Strings are told apart in this order: a URL, a `.json` file, then a bundle path.
const CLASSIFY_ORDER: [&str; 4] = ["bundle-url", "json-file", "bundle-path", "inline-map"];

/// What a manifest `mcpServers` value (or one element of an array) is, or `None`.
#[must_use]
pub fn classify_mcp_source(value: &JsValue) -> Option<McpSource> {
    CLASSIFY_ORDER
        .iter()
        .find_map(|kind| strategies(kind, value))
}

/// What decides containment (`platform`, `roots`), and the host's capabilities: a bundle opener
/// (without it a bundle is a warning) and saved bundle settings.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectOptions {
    /// Whose path rules apply; `posix` when absent.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
    /// Where the plugin and its marketplace really are.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// The host's bundle opener.
    #[serde(default)]
    pub bundles: Option<Port>,
}

/// Every server of a plugin.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectedMcpServers {
    /// The servers, by name.
    pub servers: JsObject,
    /// The bundled servers among `servers`, by name: a later source's server of the same name drops it.
    pub bundles: IndexMap<String, McpBundleConfig>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
    /// An inline manifest entry could not be read: the plugin does not load (oracle 2.1.319).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejects_plugin: Option<bool>,
}

fn unknown_source(value: &JsValue) -> McpConfigRead {
    let shown = json_stringify(value).unwrap_or_else(|| string_of(value));
    failed(
        FIELD,
        &format!(
            "{FIELD} entry must be a .json path, a bundle path or URL, or an inline map: {shown}"
        ),
    )
}

/// The plugin file a string source names, root-relative; `None` for a URL or an inline map.
fn file_of(value: &JsValue, platform: PathPlatform) -> Option<String> {
    value
        .as_str()
        .filter(|text| !is_web_url(text))
        .map(|text| root_relative(text, platform))
}

/// A file that is already a source loads once (judgment, as for the other config files): a later
/// entry naming the same file is left out.
fn without_repeated_files(declared: &[JsValue], platform: PathPlatform) -> Vec<&JsValue> {
    let mut seen = IndexSet::new();
    declared
        .iter()
        .filter(|value| file_of(value, platform).is_none_or(|file| seen.insert(file)))
        .collect()
}

/// Each server's bundle settings, read in source order, so a later server of the same name drops them.
fn bundles_of(loaded: &[McpConfigRead]) -> IndexMap<String, McpBundleConfig> {
    let mut by_name: IndexMap<String, Option<McpBundleConfig>> = IndexMap::new();
    for read in loaded {
        for name in read.servers.keys() {
            let bundle = read.bundles.as_ref().and_then(|bundles| bundles.get(name));
            by_name.insert(name.clone(), bundle.cloned());
        }
    }
    let kept = by_name
        .into_iter()
        .filter_map(|(name, bundle)| bundle.map(|bundle| (name, bundle)));
    in_key_order(kept.collect())
}

/// Every server of a plugin: the root `.mcp.json` first, then each declared source in array order;
/// a later name replaces an earlier one (a manifest server replaces a `.mcp.json` one,
/// components.md). A source that fails leaves the others loading.
pub async fn collect_mcp_servers(
    files: Port,
    manifest_value: &JsValue,
    options: CollectOptions,
) -> CollectedMcpServers {
    let context = McpLoadContext {
        files,
        platform: options.platform.unwrap_or(PathPlatform::Posix),
        roots: options.roots,
        signal: options.signal,
        bundles: options.bundles,
    };
    let declared = match manifest_value {
        JsValue::Array(items) => items.clone(),
        JsValue::Undefined => Vec::new(),
        one @ (JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Object(_)) => vec![one.clone()],
    };
    let once = without_repeated_files(&declared, context.platform);
    // One source at a time, so the host sees its files read and its bundles opened in the order
    // they are declared: run together, each one's turn would depend on how many steps its path
    // takes to resolve.
    let mut loaded = vec![load_json_file(ROOT_FILE, &context, true).await];
    for value in once {
        if file_of(value, context.platform).as_deref() == Some(ROOT_FILE) {
            continue;
        }
        loaded.push(match classify_mcp_source(value) {
            Some(source) => source.load(&context).await,
            None => unknown_source(value),
        });
    }
    let mut servers = JsObject::new();
    for read in &loaded {
        servers.extend(read.servers.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    CollectedMcpServers {
        servers: in_property_order(servers),
        bundles: bundles_of(&loaded),
        issues: loaded.iter().flat_map(|one| one.issues.clone()).collect(),
        rejects_plugin: loaded
            .iter()
            .any(|one| one.rejects_plugin == Some(true))
            .then_some(true),
    }
}

/// `McpSource` as the host holds it: its kind, and `load` a function of the core's.
#[derive(Debug, Serialize)]
pub struct McpSourceHeld {
    /// One of `MCP_SOURCE_KINDS`.
    pub kind: &'static str,
    /// `load(context)`: a promise of what it declares.
    pub load: Function,
}

/// The `load` of a source the host holds.
struct Loader(McpSource);

crate::registry::methods! {
    Loader {
        "call" => ok |this, context: McpLoadContext| {
            let source = this.0.clone();
            CorePromise::new(async move { source.load(&context).await })
        };
    }
}

fn held(source: McpSource) -> McpSourceHeld {
    McpSourceHeld {
        kind: source.kind(),
        load: Function::new(Loader(source)),
    }
}

crate::registry::exports! {
    "format/mcp-sources" {
        "MCP_SOURCE_KINDS" => ok || MCP_SOURCE_KINDS;
        "classifyMcpSource" => ok |value: JsValue| classify_mcp_source(&value).map(held);
        "collectMcpServers" => async ok |files: Port, manifest_value: JsValue, options: Option<CollectOptions>|
            collect_mcp_servers(files, &manifest_value, options.unwrap_or_default()).await;
    }
}
