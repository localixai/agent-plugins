//! Port of `src/format/layout.ts`, switched.
//!
//! The standard layout (plugins/manifest-reference.md, "Standard layout"): the default location of
//! each component under the plugin root, the manifest key that points elsewhere, and how that key
//! combines with the default ("How each key combines with its default location"). Discovery reads
//! this registry and nothing else, so a component kind added here fails the typecheck wherever it
//! needs handling.

use serde::{Deserialize, Serialize};

use crate::js::{JsValue, is_truthy};

/// A component kind. `COMPONENT_KINDS` lists them in the order the registry holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    /// `.claude-plugin/plugin.json`.
    Manifest,
    /// `skills`.
    Skills,
    /// `commands`.
    Commands,
    /// `agents`.
    Agents,
    /// `hooks/hooks.json`.
    Hooks,
    /// `.mcp.json`.
    McpServers,
    /// `.lsp.json`.
    LspServers,
    /// `output-styles`.
    OutputStyles,
    /// `workflows`.
    Workflows,
    /// `themes`.
    Themes,
    /// `monitors/monitors.json`.
    Monitors,
    /// `bin`.
    Bin,
    /// `settings.json`.
    Settings,
}

/// Every component kind.
pub const COMPONENT_KINDS: [ComponentKind; 13] = [
    ComponentKind::Manifest,
    ComponentKind::Skills,
    ComponentKind::Commands,
    ComponentKind::Agents,
    ComponentKind::Hooks,
    ComponentKind::McpServers,
    ComponentKind::LspServers,
    ComponentKind::OutputStyles,
    ComponentKind::Workflows,
    ComponentKind::Themes,
    ComponentKind::Monitors,
    ComponentKind::Bin,
    ComponentKind::Settings,
];

/// `replace`: the key replaces the default. `add`: both load. `merge`: the default loads first, the key merges into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CombineMode {
    /// The key replaces the default.
    Replace,
    /// Both load.
    Add,
    /// The default loads first, the key merges into it.
    Merge,
}

/// Every combine mode.
pub const COMBINE_MODES: [CombineMode; 3] =
    [CombineMode::Replace, CombineMode::Add, CombineMode::Merge];

/// How a manifest key is read: the value, `undefined` (`None`) when the manifest has none.
type ReadKey = fn(&JsValue) -> Option<&JsValue>;

/// A manifest key that names a component, written as the docs write it (`experimental.themes`).
/// Its reader is a Rust function, so a table of them never crosses the boundary: `STANDARD_LAYOUT`
/// crosses as the key alone, and `declared_key` reads.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ManifestKey {
    /// The key as the docs write it.
    pub key: &'static str,
    /// Reads the key's value from a manifest.
    #[serde(skip)]
    pub read: ReadKey,
}

/// One kind's place in the layout.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutEntry {
    /// Relative to the plugin root, `/`-separated, no `./` and no trailing slash.
    pub location: &'static str,
    /// Whether the default location is a directory (false: a file).
    pub directory: bool,
    /// The folder the "Default `<folder>/` folder is ignored" warning names, for a key that replaces the
    /// default; `undefined` for a kind that cannot be ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<&'static str>,
    /// The manifest keys that point elsewhere, the first one set wins.
    pub manifest_keys: &'static [ManifestKey],
    /// `undefined`: the kind has no path key that combines with a default (manifest, bin, settings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combine: Option<CombineMode>,
    /// The extension a scan of a directory of this kind keeps, `undefined` for any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<&'static str>,
}

const fn key(name: &'static str, read: ReadKey) -> ManifestKey {
    ManifestKey { key: name, read }
}

/// The registry: one entry per component kind, in `COMPONENT_KINDS`' order.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardLayout {
    manifest: LayoutEntry,
    skills: LayoutEntry,
    commands: LayoutEntry,
    agents: LayoutEntry,
    hooks: LayoutEntry,
    mcp_servers: LayoutEntry,
    lsp_servers: LayoutEntry,
    output_styles: LayoutEntry,
    workflows: LayoutEntry,
    themes: LayoutEntry,
    monitors: LayoutEntry,
    bin: LayoutEntry,
    settings: LayoutEntry,
}

impl StandardLayout {
    /// `STANDARD_LAYOUT[kind]`.
    #[must_use]
    pub const fn of(&self, kind: ComponentKind) -> &LayoutEntry {
        match kind {
            ComponentKind::Manifest => &self.manifest,
            ComponentKind::Skills => &self.skills,
            ComponentKind::Commands => &self.commands,
            ComponentKind::Agents => &self.agents,
            ComponentKind::Hooks => &self.hooks,
            ComponentKind::McpServers => &self.mcp_servers,
            ComponentKind::LspServers => &self.lsp_servers,
            ComponentKind::OutputStyles => &self.output_styles,
            ComponentKind::Workflows => &self.workflows,
            ComponentKind::Themes => &self.themes,
            ComponentKind::Monitors => &self.monitors,
            ComponentKind::Bin => &self.bin,
            ComponentKind::Settings => &self.settings,
        }
    }
}

/// The standard layout.
pub const STANDARD_LAYOUT: StandardLayout = StandardLayout {
    manifest: LayoutEntry {
        location: ".claude-plugin/plugin.json",
        directory: false,
        folder: None,
        manifest_keys: &[],
        combine: None,
        extension: Some(".json"),
    },
    skills: LayoutEntry {
        location: "skills",
        directory: true,
        folder: None,
        manifest_keys: &[key("skills", |manifest| manifest.get("skills"))],
        combine: Some(CombineMode::Add),
        extension: None,
    },
    commands: LayoutEntry {
        location: "commands",
        directory: true,
        folder: Some("commands"),
        manifest_keys: &[key("commands", |manifest| manifest.get("commands"))],
        combine: Some(CombineMode::Replace),
        extension: Some(".md"),
    },
    agents: LayoutEntry {
        location: "agents",
        directory: true,
        folder: Some("agents"),
        manifest_keys: &[key("agents", |manifest| manifest.get("agents"))],
        combine: Some(CombineMode::Replace),
        extension: Some(".md"),
    },
    hooks: LayoutEntry {
        location: "hooks/hooks.json",
        directory: false,
        folder: None,
        manifest_keys: &[key("hooks", |manifest| manifest.get("hooks"))],
        combine: Some(CombineMode::Merge),
        extension: Some(".json"),
    },
    mcp_servers: LayoutEntry {
        location: ".mcp.json",
        directory: false,
        folder: None,
        manifest_keys: &[key("mcpServers", |manifest| manifest.get("mcpServers"))],
        combine: Some(CombineMode::Merge),
        extension: Some(".json"),
    },
    lsp_servers: LayoutEntry {
        location: ".lsp.json",
        directory: false,
        folder: None,
        manifest_keys: &[key("lspServers", |manifest| manifest.get("lspServers"))],
        combine: Some(CombineMode::Merge),
        extension: Some(".json"),
    },
    output_styles: LayoutEntry {
        location: "output-styles",
        directory: true,
        folder: Some("output-styles"),
        manifest_keys: &[key("outputStyles", |manifest| manifest.get("outputStyles"))],
        combine: Some(CombineMode::Replace),
        extension: Some(".md"),
    },
    workflows: LayoutEntry {
        location: "workflows",
        directory: true,
        folder: Some("workflows"),
        manifest_keys: &[key("workflows", |manifest| manifest.get("workflows"))],
        combine: Some(CombineMode::Replace),
        extension: Some(".js"),
    },
    themes: LayoutEntry {
        location: "themes",
        directory: true,
        folder: Some("themes"),
        manifest_keys: &[
            key("experimental.themes", |manifest| {
                manifest
                    .get("experimental")
                    .and_then(|experimental| experimental.get("themes"))
            }),
            key("themes", |manifest| manifest.get("themes")),
        ],
        combine: Some(CombineMode::Replace),
        extension: Some(".json"),
    },
    monitors: LayoutEntry {
        location: "monitors/monitors.json",
        directory: false,
        folder: Some("monitors"),
        manifest_keys: &[
            key("experimental.monitors", |manifest| {
                manifest
                    .get("experimental")
                    .and_then(|experimental| experimental.get("monitors"))
            }),
            key("monitors", |manifest| manifest.get("monitors")),
        ],
        combine: Some(CombineMode::Replace),
        extension: Some(".json"),
    },
    bin: LayoutEntry {
        location: "bin",
        directory: true,
        folder: None,
        manifest_keys: &[],
        combine: None,
        extension: None,
    },
    settings: LayoutEntry {
        location: "settings.json",
        directory: false,
        folder: None,
        manifest_keys: &[key("settings", |manifest| manifest.get("settings"))],
        combine: None,
        extension: Some(".json"),
    },
};

/// The first manifest key of a kind that is set, with its value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DeclaredKey {
    /// The key as the docs write it.
    pub key: &'static str,
    /// What it holds.
    pub value: JsValue,
}

/// The first manifest key of a kind that is set, with its value; `undefined` when none is.
#[must_use]
pub fn declared_key(manifest: &JsValue, kind: ComponentKind) -> Option<DeclaredKey> {
    if !is_truthy(manifest) {
        return None;
    }
    for candidate in STANDARD_LAYOUT.of(kind).manifest_keys {
        if let Some(value) = (candidate.read)(manifest).filter(|value| !value.is_undefined()) {
            return Some(DeclaredKey {
                key: candidate.key,
                value: value.clone(),
            });
        }
    }
    None
}

/// The first segment of a kind's default location: what a plugin root holds for that kind.
fn top_level_of(kind: ComponentKind) -> Option<&'static str> {
    STANDARD_LAYOUT.of(kind).location.split('/').next()
}

/// The component kinds a directory listing shows at the plugin root, by their default locations (the
/// manifest's `.claude-plugin/` folder is not a component). `names` are the entry names of the root.
#[must_use]
pub fn components_at(names: &[String]) -> Vec<ComponentKind> {
    COMPONENT_KINDS
        .into_iter()
        .filter(|kind| {
            let top = top_level_of(*kind);
            *kind != ComponentKind::Manifest
                && top.is_some_and(|top| names.iter().any(|name| name == top))
        })
        .collect()
}

crate::registry::exports! {
    "format/layout" {
        "COMPONENT_KINDS" => ok || COMPONENT_KINDS;
        "COMBINE_MODES" => ok || COMBINE_MODES;
        "STANDARD_LAYOUT" => ok || STANDARD_LAYOUT;
        "declaredKey" => ok |manifest: JsValue, kind: ComponentKind| declared_key(&manifest, kind);
        "componentsAt" => ok |names: Vec<String>| components_at(&names);
    }
}
