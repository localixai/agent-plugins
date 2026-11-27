//! Port of `src/cli/flags/customizations.ts`, switched.
//!
//! The customizations a mode leaves out (cli-reference.md, `--bare`, `--safe-mode`,
//! `--disable-slash-commands`; headless.md, "Start faster with bare mode"; env-vars.md,
//! `CLAUDE_CODE_SIMPLE` and `CLAUDE_CODE_SAFE_MODE`). `installed-plugins` is the plugins installed
//! from marketplaces: a plugin loaded by `--plugin-dir` or `--plugin-url` still loads in bare mode
//! ("A plugin: `--plugin-dir <path>`, `--plugin-url <url>`" is what to pass to load one). The docs
//! name the eight of `--bare` and the fifteen of `--safe-mode`; whatever neither names stays loaded
//! (spec/judgments.md).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::types::{CUSTOMIZATIONS, Customization};

/// `bare`, `safe-mode` or `disable-slash-commands`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipMode {
    /// `bare`.
    Bare,
    /// `safe-mode`.
    SafeMode,
    /// `disable-slash-commands`.
    DisableSlashCommands,
}

/// Every mode that skips customizations.
pub const SKIP_MODES: [SkipMode; 3] = [
    SkipMode::Bare,
    SkipMode::SafeMode,
    SkipMode::DisableSlashCommands,
];

/// What each mode skips: `(bare, safe-mode, disable-slash-commands)`.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "one flag per mode, in the order the doc line names"
)]
const fn skipped_in(bare: bool, safe_mode: bool, no_slash: bool) -> [bool; 3] {
    [bare, safe_mode, no_slash]
}

/// One row per customization, so a kind added to the union fails the build until it is placed.
const fn skipped_by_row(customization: Customization) -> [bool; 3] {
    match customization {
        Customization::Hooks => skipped_in(true, true, false),
        Customization::Skills => skipped_in(true, true, true),
        Customization::Commands => skipped_in(true, true, true),
        Customization::Agents => skipped_in(true, true, false),
        Customization::InstalledPlugins => skipped_in(true, true, false),
        Customization::McpServers => skipped_in(true, true, false),
        Customization::AutoMemory => skipped_in(true, true, false),
        Customization::ClaudeMd => skipped_in(true, true, false),
        Customization::OutputStyles => skipped_in(false, true, false),
        Customization::Workflows => skipped_in(false, true, false),
        Customization::Themes => skipped_in(false, true, false),
        Customization::Keybindings => skipped_in(false, true, false),
        Customization::StatusLine => skipped_in(false, true, false),
        Customization::FileSuggestion => skipped_in(false, true, false),
        Customization::LspServers => skipped_in(false, true, false),
    }
}

/// The customizations `mode` skips, in the order of `CUSTOMIZATIONS`.
#[must_use]
pub fn skipped_by(mode: SkipMode) -> Vec<Customization> {
    let column = match mode {
        SkipMode::Bare => 0,
        SkipMode::SafeMode => 1,
        SkipMode::DisableSlashCommands => 2,
    };
    CUSTOMIZATIONS
        .into_iter()
        .filter(|customization| skipped_by_row(*customization)[column])
        .collect()
}

/// What `--safe-mode` does with the managed layer of a customization it skips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagedLayer {
    /// `applies`.
    Applies,
    /// `skipped`.
    Skipped,
}

/// What `--safe-mode` does with the managed layer of a customization it skips: "Managed settings
/// policy still applies, including policy-configured hooks, status line, and file-suggestion
/// commands; managed plugins, managed skills, managed CLAUDE.md, and policy-configured MCP servers do
/// not." The docs say nothing of the other kinds, so they are absent.
#[must_use]
pub fn managed_under_safe_mode() -> IndexMap<String, ManagedLayer> {
    [
        ("hooks", ManagedLayer::Applies),
        ("status-line", ManagedLayer::Applies),
        ("file-suggestion", ManagedLayer::Applies),
        ("installed-plugins", ManagedLayer::Skipped),
        ("skills", ManagedLayer::Skipped),
        ("claude-md", ManagedLayer::Skipped),
        ("mcp-servers", ManagedLayer::Skipped),
    ]
    .into_iter()
    .map(|(name, layer)| (name.to_owned(), layer))
    .collect()
}

crate::registry::exports! {
    "cli/flags/customizations" {
        "SKIP_MODES" => ok || SKIP_MODES;
        "skippedBy" => ok |mode: SkipMode| skipped_by(mode);
        "MANAGED_UNDER_SAFE_MODE" => ok managed_under_safe_mode;
    }
}
