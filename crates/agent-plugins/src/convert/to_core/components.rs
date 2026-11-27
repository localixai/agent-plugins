//! Port of `src/convert/to-core/components.ts`, switched.
//!
//! What of a Claude Code plugin has no core counterpart (Agent Plugins Extended §18.4.5): commands,
//! agents, output styles, themes, LSP servers, monitors, workflows, `bin/`, `settings.json` and mods.
//! Each is reported with its path; the files stay where they are, for Claude Code (§18.4.1 item 4).

use serde::Deserialize;

use crate::convert::notes::{ConversionNote, not_converted};
use crate::convert::to_claude_code::manifest::CLAUDE_MANIFEST;
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT};
use crate::js::{JsObject, property_order};
use crate::registry::Port;

/// A loaded file of a component: its path.
#[derive(Clone, Debug, Deserialize)]
pub struct WithPath {
    /// `None` for a command whose `content` is inline in the manifest.
    #[serde(default)]
    pub path: Option<String>,
}

/// A monitor: its name.
#[derive(Clone, Debug, Deserialize)]
pub struct Named {
    /// The monitor's name.
    pub name: String,
}

/// What of a loaded `Plugin` the notes read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginComponents {
    /// Its commands.
    pub commands: Vec<WithPath>,
    /// Its agents.
    pub agents: Vec<WithPath>,
    /// Its output styles.
    pub output_styles: Vec<WithPath>,
    /// Its themes.
    pub themes: Vec<WithPath>,
    /// Its workflows.
    pub workflows: Vec<WithPath>,
    /// Its LSP servers, by name.
    pub lsp_servers: JsObject,
    /// Its monitors.
    pub monitors: Vec<Named>,
    /// Its settings.
    pub settings: JsObject,
    /// `bin/` exists.
    pub has_bin: bool,
}

/// One note of a component: what `note(path, field, message)` makes.
struct Planned {
    path: String,
    field: Option<String>,
    message: String,
}

/// The notes to make, in order; `says` follows each component's name.
fn planned(plugin: &PluginComponents, mods: bool, says: &str) -> Vec<Planned> {
    let files = |found: &[WithPath], what: &str| -> Vec<Planned> {
        found
            .iter()
            .map(|one| match &one.path {
                None => Planned {
                    path: CLAUDE_MANIFEST.to_owned(),
                    field: Some("commands".to_owned()),
                    message: format!("{what} {says}"),
                },
                Some(path) => Planned {
                    path: path.clone(),
                    field: None,
                    message: format!("{what} {says}"),
                },
            })
            .collect()
    };
    let named = |names: Vec<String>, path: &str, what: &str| -> Vec<Planned> {
        names
            .into_iter()
            .map(|name| Planned {
                path: path.to_owned(),
                message: format!("{what} \"{name}\" {says}"),
                field: Some(name),
            })
            .collect()
    };
    let lsp: Vec<String> = property_order(&plugin.lsp_servers)
        .into_iter()
        .map(|(name, _)| name.clone())
        .collect();
    let monitors: Vec<String> = plugin.monitors.iter().map(|one| one.name.clone()).collect();
    let at = |kind| STANDARD_LAYOUT.of(kind).location;
    let mut notes = files(&plugin.commands, "a command");
    notes.extend(files(&plugin.agents, "an agent"));
    notes.extend(files(&plugin.output_styles, "an output style"));
    notes.extend(files(&plugin.themes, "a theme"));
    notes.extend(files(&plugin.workflows, "a workflow"));
    notes.extend(named(lsp, at(ComponentKind::LspServers), "the LSP server"));
    notes.extend(named(monitors, at(ComponentKind::Monitors), "the monitor"));
    let whole = |path: &str, field: Option<&str>, what: &str| Planned {
        path: path.to_owned(),
        field: field.map(str::to_owned),
        message: format!("{what} {says}"),
    };
    if plugin.has_bin {
        notes.push(whole("bin/", None, "`bin/`"));
    }
    if !plugin.settings.is_empty() {
        notes.push(whole(at(ComponentKind::Settings), None, "plugin settings"));
    }
    if mods {
        notes.push(whole(at(ComponentKind::Hooks), Some("modules"), "a mod"));
    }
    notes
}

const NOT_IN_CORE_SAYS: &str = "has no core counterpart";

/// One note per component; `mods`: the plugin's hooks file declares `modules`, which `Plugin` does not
/// carry.
#[must_use]
pub fn component_notes(plugin: &PluginComponents, mods: bool) -> Vec<ConversionNote> {
    planned(plugin, mods, NOT_IN_CORE_SAYS)
        .into_iter()
        .map(|one| not_converted(one.path, one.field, one.message))
        .collect()
}

/// `componentNotes` with the host's wording (`ComponentWording`): a record holding the note's maker
/// (`notConverted`, `reported`), so it crosses as a port; `says` is its member, and each note is
/// made by calling its `note`.
pub async fn component_notes_worded(
    plugin: &PluginComponents,
    mods: bool,
    wording: Port,
) -> Vec<ConversionNote> {
    let says: String = wording.member("says").await;
    let mut notes = Vec::new();
    for one in planned(plugin, mods, &says) {
        let field = one.field.map_or(crate::js::JsValue::Undefined, Into::into);
        let args = vec![one.path.into(), field, one.message.into()];
        notes.push(wording.method::<ConversionNote>("note", args).await);
    }
    notes
}

crate::registry::exports! {
    "convert/to-core/components" {
        "componentNotes" => async ok |plugin: PluginComponents, mods: bool, wording: Option<Port>|
            match wording {
                None => component_notes(&plugin, mods),
                Some(wording) => component_notes_worded(&plugin, mods, wording).await,
            };
    }
}
