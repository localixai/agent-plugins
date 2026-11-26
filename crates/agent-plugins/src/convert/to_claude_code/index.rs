//! Port of `src/convert/to-claude-code/index.ts`, switched.
//!
//! A core package as a Claude Code plugin (Agent Plugins Extended §18.4.2), pure over the loaded
//! package: the files to write beside it, which make a dual package (§18.4.1 item 4), and a note for
//! everything not carried over (item 1). The extension data the package was loaded with is
//! reported, so a host loads it with every namespace it holds (`convertPluginPackage` does) to see
//! all of it.

use indexmap::IndexMap;
use serde::Deserialize;

use super::hooks::{CLAUDE_HOOKS, claude_code_hooks};
use super::manifest::{CLAUDE_MANIFEST, claude_code_manifest};
use super::mcp::{CLAUDE_MCP, claude_code_mcp};
use super::skills::claude_code_skill_notes;
use crate::convert::adapters::registration::RUN_CORE_HANDLER;
use crate::convert::adapters::sources::{ADAPTER_RUNTIME, RUN_CORE_HANDLER_SOURCE};
use crate::convert::notes::{Conversion, ConvertedFile, conversion, json_file, not_converted};
use crate::format::core::hooks::CoreHooks;
use crate::format::core::manifest::CoreManifest;
use crate::format::core::mcp_server::CoreMcpServer;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::loading::readers_text::PluginTextFile;

/// What the conversion reads of a loaded core package (`CorePackage`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorePackage {
    /// The manifest.
    pub manifest: CoreManifest,
    /// One per conforming `skills/<directory>/SKILL.md`.
    pub skills: Vec<PluginTextFile>,
    /// The servers, by name.
    pub mcp_servers: IndexMap<String, CoreMcpServer>,
    /// The handlers, by event.
    pub hooks: CoreHooks,
    /// The top-level directories named for an implemented extension namespace (core §8.2).
    pub extension_directories: Vec<String>,
}

/// How a conversion runs (`ConvertOptions`).
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    /// Route every hook handler through a hook adapter (§18.4.4), the default.
    #[serde(default)]
    pub hook_adapter: Option<bool>,
}

/// A JSON file of one member.
fn wrapped<T: serde::Serialize>(path: &str, key: &str, value: &T) -> ConvertedFile {
    let mut members = JsObject::new();
    members.insert(
        key.to_owned(),
        to_js_value(value).unwrap_or(JsValue::Undefined),
    );
    json_file(path.to_owned(), &JsValue::Object(members))
}

/// A core package as a Claude Code plugin.
#[must_use]
pub fn core_to_claude_code(source: &CorePackage, options: ConvertOptions) -> Conversion {
    let manifest = claude_code_manifest(&source.manifest);
    let mcp = claude_code_mcp(&source.mcp_servers);
    let hooks = claude_code_hooks(&source.hooks, options.hook_adapter.unwrap_or(true));
    let mut files = vec![json_file(
        CLAUDE_MANIFEST.to_owned(),
        &JsValue::Object(manifest.value),
    )];
    if !mcp.servers.is_empty() {
        files.push(wrapped(CLAUDE_MCP, "mcpServers", &mcp.servers));
    }
    if !hooks.hooks.is_empty() {
        files.push(wrapped(CLAUDE_HOOKS, "hooks", &hooks.hooks));
    }
    if hooks.adapted {
        files.push(ConvertedFile {
            path: RUN_CORE_HANDLER.to_owned(),
            text: RUN_CORE_HANDLER_SOURCE.clone(),
        });
    }
    let directories = source.extension_directories.iter().map(|directory| {
        not_converted(
            format!("{directory}/"),
            None,
            "an extension directory has no Claude Code counterpart".to_owned(),
        )
    });
    let mut notes = manifest.notes;
    notes.extend(claude_code_skill_notes(&source.skills));
    notes.extend(mcp.notes);
    notes.extend(hooks.notes);
    notes.extend(directories);
    let runtime = hooks.adapted.then(|| ADAPTER_RUNTIME.to_owned());
    conversion("claude-code".to_owned(), files, notes, runtime)
}

crate::registry::exports! {
    "convert/to-claude-code/index" {
        "coreToClaudeCode" => ok |source: CorePackage, options: Option<ConvertOptions>|
            core_to_claude_code(&source, options.unwrap_or_default());
    }
}
