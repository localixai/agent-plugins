//! Port of `src/format/mcp-bundles.ts`, switched.
//!
//! MCP bundles (plugins/manifest-reference.md, "mcpServers"; plugins/components.md, "Include a
//! packaged MCPB server"). The host opens a bundle (extracts or downloads it into `.mcpb-cache/`
//! under the plugin root, reads its manifest into a server config); the kit decides what a manifest
//! value is, that it ends in `.mcpb` or `.dxt`, which server name results and what the bundle's
//! settings are. Whether the server may start and what its `${user_config.KEY}` references stand for
//! are decided when it is resolved, from the session's saved values (`loading/resolve/bundle-scope.ts`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error, warning};
use super::locate::LocateContext;
use super::mcp_config::{McpBundleConfig, McpConfigRead, read_mcp_server};
use super::mcpb_manifest::read_mcpb_options;
use super::messages::path_not_found;
use super::path_escape::LinkRoots;
use super::path_text::PathPlatform;
use crate::js::JsObject;
use crate::js::value::to_js_value;
use crate::ports::mcp_bundle::{McpBundle, McpBundleReader, McpBundleSource};
use crate::ports::{PortFailure, Signal};
use crate::registry::Port;

/// Where the host puts an extracted or downloaded bundle, relative to the plugin root.
const MCPB_CACHE_DIR: &str = ".mcpb-cache";

/// What a source needs to load: the plugin's files and what decides whether a path leaves them
/// (`locate`), and, optionally, the host's capabilities.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoadContext {
    /// The plugin folder.
    pub files: Port,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Where the plugin and its marketplace really are.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// Absent: a bundle is a warning, never a crash.
    #[serde(default)]
    pub bundles: Option<Port>,
}

impl McpLoadContext {
    /// The `LocateContext` it extends.
    #[must_use]
    pub fn located(&self) -> LocateContext {
        LocateContext {
            files: self.files,
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
        }
    }
}

const BUNDLE_EXTENSIONS: [&str; 2] = [".mcpb", ".dxt"];
const FIELD: &str = "mcpServers";

fn has_bundle_extension(value: &str) -> bool {
    BUNDLE_EXTENSIONS
        .iter()
        .any(|extension| value.ends_with(extension))
}

fn nothing(issue: PluginIssue) -> McpConfigRead {
    McpConfigRead {
        servers: JsObject::new(),
        bundles: None,
        issues: vec![issue],
        rejects_plugin: None,
    }
}

/// The host's answer to `open`, as servers and issues.
fn bundled_server(value: &str, opened: Option<Result<McpBundle, PortFailure>>) -> McpConfigRead {
    let opened = match opened {
        None => return nothing(error(FIELD, &path_not_found(FIELD, value))),
        Some(Err(failure)) => {
            return nothing(error(
                FIELD,
                &format!(
                    "MCP bundle {value} could not be opened: {}",
                    failure.message
                ),
            ));
        }
        Some(Ok(opened)) => opened,
    };
    let read = read_mcp_server(&opened.name, &opened.server, FIELD);
    let Some(server) = read.server else {
        return McpConfigRead {
            servers: JsObject::new(),
            bundles: None,
            issues: read.issues,
            rejects_plugin: None,
        };
    };
    let user_config = to_js_value(&read_mcpb_options(&opened.user_config))
        .ok()
        .and_then(|options| options.as_object().cloned())
        .unwrap_or_default();
    McpConfigRead {
        servers: JsObject::from([(opened.name.clone(), server)]),
        bundles: Some(IndexMap::from([(
            opened.name,
            McpBundleConfig { user_config },
        )])),
        issues: read.issues,
        rejects_plugin: None,
    }
}

/// Loads one bundle. `source` is root-relative (`PluginFiles` paths), `value` the manifest text.
/// Judgments: the extension is matched exactly (`.mcpb`, `.dxt`), a URL must be `https://`, and the
/// host is never asked for a value that fails either check.
pub async fn load_bundle(
    value: &str,
    source: &McpBundleSource,
    context: &McpLoadContext,
) -> McpConfigRead {
    if !has_bundle_extension(value) {
        return nothing(error(
            FIELD,
            &format!("{FIELD} bundle must end in .mcpb or .dxt: {value}"),
        ));
    }
    if matches!(source, McpBundleSource::Url { .. }) && !value.starts_with("https://") {
        return nothing(error(
            FIELD,
            &format!("{FIELD} bundle URL must use https://: {value}"),
        ));
    }
    let Some(bundles) = &context.bundles else {
        let missing = warning(
            FIELD,
            &format!("MCP bundle {value} needs an extractor: the host gave none"),
        );
        return nothing(missing);
    };
    let opened = bundles
        .open(source, MCPB_CACHE_DIR, context.signal.as_ref())
        .await;
    bundled_server(value, opened)
}

/// One `--config` flag of `claude plugin install`: a plugin option, or a setting of a bundled server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConfigFlag {
    /// The bundled server, for a key written `<server>.<key>`; absent for a plugin `userConfig` option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// The option or setting.
    pub key: String,
    /// Its value.
    pub value: String,
}

/// A flag, or why it is no flag.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ConfigFlagRead {
    /// The flag.
    Flag(ConfigFlag),
    /// What is wrong with it.
    Issue(PluginIssue),
}

/// `--config <key=value>` (plugins/cli-reference.md, "plugin install"): "A key written `<server>.<key>`
/// sets a setting that a bundled MCP server declares in its own `user_config` instead". Judgment
/// (spec/judgments.md): the first `=` ends the key and the first `.` of the key ends the server name,
/// so a value keeps its own `=` and a setting key may hold dots; a flag with no `=` or an empty part is
/// an error.
#[must_use]
pub fn read_config_flag(text: &str) -> ConfigFlagRead {
    let (written, value) = text.split_once('=').unwrap_or(("", ""));
    let (server, key) = match written.split_once('.') {
        None => (None, written),
        Some((server, key)) => (Some(server), key),
    };
    if key.is_empty() || server == Some("") {
        return ConfigFlagRead::Issue(error(
            "--config",
            &format!("--config takes key=value or <server>.<key>=value, not \"{text}\""),
        ));
    }
    ConfigFlagRead::Flag(ConfigFlag {
        server: server.map(str::to_owned),
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

/// "For a bundle file shipped inside the plugin": a `<server>.<key>` flag works only when the bundle is
/// a path in the plugin, never for one downloaded from a URL (`CONFIG_FLAG_APPLIES`).
#[must_use]
pub const fn config_flag_applies(source: &McpBundleSource) -> bool {
    match source {
        McpBundleSource::Path { .. } => true,
        McpBundleSource::Url { .. } => false,
    }
}

crate::registry::exports! {
    "format/mcp-bundles" {
        "readConfigFlag" => ok |text: String| read_config_flag(&text);
        "configFlagApplies" => ok |source: McpBundleSource| config_flag_applies(&source);
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigFlagRead, read_config_flag};

    #[test]
    fn a_flag_splits_at_its_first_equals_and_dot() {
        let ConfigFlagRead::Flag(flag) = read_config_flag("db.url=a=b") else {
            panic!("no flag")
        };
        assert_eq!(
            (
                flag.server.as_deref(),
                flag.key.as_str(),
                flag.value.as_str()
            ),
            (Some("db"), "url", "a=b")
        );
        assert!(matches!(read_config_flag(".k=v"), ConfigFlagRead::Issue(_)));
        assert!(matches!(read_config_flag("k"), ConfigFlagRead::Issue(_)));
    }
}
