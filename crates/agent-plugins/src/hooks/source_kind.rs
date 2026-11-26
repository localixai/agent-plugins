//! Port of `src/hooks/source-kind.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::contracts::{HookSource, HookSourceKind};

/// What a kind of declaration site decides about its handlers, wherever the hooks read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one row of the package's policy table: five independent yes-or-no facts of a kind"
)]
pub struct SourcePolicy {
    /// A plugin's hooks file.
    pub plugin: bool,
    /// hooks.md, "Hook handler fields": "If you define the same handler in more than one settings
    /// file, it runs once. A plugin's or skill's copy of the same handler stays separate."
    pub merges_copies: bool,
    /// hooks.md, "Common fields": `once` is "Only honored for hooks declared in skill frontmatter;
    /// ignored in settings files and agent frontmatter" (and so in a plugin's hooks file).
    pub honors_once: bool,
    /// `SessionEnd`: "your settings files" raise the budget, "plugin-provided hooks" do not (the
    /// other kinds: judgment, spec/judgments.md).
    pub raises_session_end_budget: bool,
    /// A session registered it after it opened (a skill's, an agent's, the host's run): an update
    /// keeps it, while the plugin and settings handlers are rebuilt from the new state.
    pub scoped: bool,
}

/// The row of a kind.
const fn policy_of(kind: HookSourceKind) -> SourcePolicy {
    let (plugin, merges_copies, honors_once, raises_session_end_budget, scoped) = match kind {
        HookSourceKind::Plugin => (true, false, false, false, false),
        HookSourceKind::Settings => (false, true, false, true, false),
        HookSourceKind::Skill => (false, false, true, false, true),
        HookSourceKind::Agent | HookSourceKind::Session => (false, false, false, false, true),
    };
    SourcePolicy {
        plugin,
        merges_copies,
        honors_once,
        raises_session_end_budget,
        scoped,
    }
}

/// The kinds, in the order of the package's table.
const KINDS: [(&str, HookSourceKind); 5] = [
    ("plugin", HookSourceKind::Plugin),
    ("settings", HookSourceKind::Settings),
    ("skill", HookSourceKind::Skill),
    ("agent", HookSourceKind::Agent),
    ("session", HookSourceKind::Session),
];

/// One row per kind: a registry over the union, so a new kind is a compile error here.
#[must_use]
pub fn source_policy() -> IndexMap<&'static str, SourcePolicy> {
    KINDS
        .iter()
        .map(|(name, kind)| (*name, policy_of(*kind)))
        .collect()
}

/// Whether the source is a plugin's hooks file.
#[must_use]
pub const fn is_plugin_source(source: &HookSource) -> bool {
    policy_of(source.kind).plugin
}

/// The part of a registered handler the label reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelledEntry {
    /// The plugin's full id.
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// Where the handler was declared.
    pub source: HookSource,
}

/// The name a plugin hook's notices give its plugin: the manifest name, else the plugin id. One
/// rule for the block error ("This hook comes from the `<plugin>` plugin.") and the missing
/// directory notice.
#[must_use]
pub fn plugin_label(entry: &LabelledEntry) -> Option<String> {
    entry
        .source
        .plugin_name
        .clone()
        .or_else(|| entry.plugin_id.clone())
}

crate::registry::exports! {
    "hooks/source-kind" {
        "SOURCE_POLICY" => ok source_policy;
        "isPluginSource" => ok |source: HookSource| is_plugin_source(&source);
        "pluginLabel" => ok |entry: LabelledEntry| plugin_label(&entry);
    }
}
