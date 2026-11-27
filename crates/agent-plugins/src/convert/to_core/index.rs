//! Port of `src/convert/to-core/index.ts`, switched.
//!
//! A Claude Code plugin as a core package (Agent Plugins Extended §18.4.3), pure over the loaded
//! plugin: `plugin.json`, `mcp.json` and `hooks.json` to write beside it, making a dual package
//! (§18.4.1 item 4), and a note for everything §18.4.5 lists as having no core counterpart. A name
//! core §5.5 refuses stops the conversion.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::components::{PluginComponents, component_notes};
use super::hooks::{CORE_HOOKS, HooksSettings, core_hooks_of};
use super::manifest::{CORE_MANIFEST, PluginNaming, core_manifest_of};
use super::mcp::{CORE_MCP, core_mcp_of};
use super::skills::core_skill_notes;
use crate::convert::adapters::registration::RUN_CLAUDE_CODE_HANDLER;
use crate::convert::adapters::sources::{ADAPTER_RUNTIME, RUN_CLAUDE_CODE_HANDLER_SOURCE};
use crate::convert::notes::{Conversion, ConvertedFile, conversion, json_file};
use crate::format::core::versions::{CoreDocument, CoreVersion, core_schema_id};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::loading::readers_text::PluginTextFile;

/// What the conversion reads of a loaded `Plugin` beside its naming and its components.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginParts {
    skills: Vec<PluginTextFile>,
    mcp_servers: JsObject,
    mcp_bundles: JsObject,
    hooks: HooksSettings,
}

/// What a conversion to the core form is told.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeCodeToCoreOptions {
    /// Whether handlers the core cannot run go through the adapter (`ConvertOptions`).
    #[serde(default)]
    pub hook_adapter: Option<bool>,
    /// The plugin's `hooks/hooks.json` declares `modules` (a mod), which `Plugin` does not carry; it is
    /// reported as not converted. `convertPluginPackage` reads it.
    #[serde(default)]
    pub mods: Option<bool>,
}

fn view<T: DeserializeOwned>(source: &JsValue) -> Result<T, String> {
    T::deserialize(source.clone()).map_err(|problem| format!("the plugin does not read: {problem}"))
}

/// `{ $schema, <key>: value }`.
fn schema_file(path: &str, document: CoreDocument, key: &str, value: JsValue) -> ConvertedFile {
    let mut members = JsObject::new();
    let id = core_schema_id(CoreVersion::V1_1_0, document);
    members.insert("$schema".to_owned(), id.into());
    members.insert(key.to_owned(), value);
    json_file(path.to_owned(), &JsValue::Object(members))
}

/// The plugin as a core package.
///
/// # Errors
///
/// A value that is no loaded `Plugin`.
pub fn claude_code_to_core(
    source: &JsValue,
    options: ClaudeCodeToCoreOptions,
) -> Result<Conversion, String> {
    let naming: PluginNaming = view(source)?;
    let parts: PluginParts = view(source)?;
    let components: PluginComponents = view(source)?;
    let manifest = core_manifest_of(&naming);
    let bundles: Vec<String> = parts.mcp_bundles.keys().cloned().collect();
    let mcp = core_mcp_of(&parts.mcp_servers, &bundles);
    let hooks = core_hooks_of(&parts.hooks, options.hook_adapter.unwrap_or(true));
    let mut files = vec![json_file(
        CORE_MANIFEST.to_owned(),
        &JsValue::Object(manifest.value),
    )];
    if !mcp.servers.is_empty() {
        let servers = to_js_value(&mcp.servers).unwrap_or_default();
        files.push(schema_file(
            CORE_MCP,
            CoreDocument::Mcp,
            "mcpServers",
            servers,
        ));
    }
    if !hooks.hooks.is_empty() {
        let handlers = to_js_value(&hooks.hooks).unwrap_or_default();
        files.push(schema_file(
            CORE_HOOKS,
            CoreDocument::Hooks,
            "hooks",
            handlers,
        ));
    }
    if hooks.adapted {
        files.push(ConvertedFile {
            path: RUN_CLAUDE_CODE_HANDLER.to_owned(),
            text: RUN_CLAUDE_CODE_HANDLER_SOURCE.clone(),
        });
    }
    let mut notes = manifest.notes;
    notes.extend(core_skill_notes(&parts.skills));
    notes.extend(mcp.notes);
    notes.extend(hooks.notes);
    notes.extend(component_notes(&components, options.mods == Some(true)));
    let runtime = hooks.adapted.then(|| ADAPTER_RUNTIME.to_owned());
    Ok(conversion("core".to_owned(), files, notes, runtime))
}

crate::registry::exports! {
    "convert/to-core/index" {
        "claudeCodeToCore" => fallible |source: JsValue, options: Option<ClaudeCodeToCoreOptions>|
            claude_code_to_core(&source, options.unwrap_or_default());
    }
}
