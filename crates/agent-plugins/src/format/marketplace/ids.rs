//! Port of `src/format/marketplace/ids.ts`, switched.
//!
//! Each part of a plugin id, `plugin@marketplace` (plugins/marketplace-reference.md, "Validation
//! messages"; plugins/troubleshooting.md, "Cannot add marketplace"): letters a-z and A-Z, digits,
//! `.`, `_` and `-`, starting with a letter or digit. Shared by entries, rename targets and
//! dependencies, so the rule exists once.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error, warning};
use crate::format::names::contains_control_or_bidi;
use crate::js::{Pattern, code_unit_length, regex_test};

/// The grammar of one part of a plugin id (RS6: ASCII classes).
static PLUGIN_ID_PART: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]*$").ok());

/// Whether `name` is one part of a plugin id.
#[must_use]
pub fn is_plugin_id_part(name: &str) -> bool {
    regex_test(&PLUGIN_ID_PART, name)
}

/// Which part of a plugin id a name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdPart {
    /// The part after the `@`.
    Marketplace,
    /// The part before it.
    Plugin,
}

// The control characters the docs name ("an escape or a newline") and Unicode's
// bidirectional-formatting set: `names::contains_control_or_bidi`.

/// Marketplace `name` also may not hold a path separator or `..`, or be `.` (the "Top-level fields" `name` row).
fn has_marketplace_path(name: &str) -> bool {
    name.contains(['/', '\\']) || name.contains("..")
}

const MARKETPLACE_PATH_MESSAGE: &str =
    "Marketplace name cannot contain path separators (/ or \\), \"..\" sequences, or be \".\"";

const fn label(part: IdPart) -> &'static str {
    match part {
        IdPart::Marketplace => "Marketplace",
        IdPart::Plugin => "Plugin",
    }
}

const fn kebab_example(part: IdPart) -> &'static str {
    match part {
        IdPart::Marketplace => "my-marketplace",
        IdPart::Plugin => "my-plugin",
    }
}

/// The rule a name breaks when Claude Code cannot install it.
pub const INSTALL_RULE: &str = "Each part of a plugin id (plugin@marketplace) may use only the letters a-z and A-Z, digits, \".\", \"_\" and \"-\", and must start with a letter or digit.";

fn install_message(part: IdPart, name: &str) -> String {
    match part {
        IdPart::Marketplace => format!(
            "Claude Code cannot install plugins from marketplace \"{name}\". {INSTALL_RULE} Change the marketplace's \"name\"."
        ),
        IdPart::Plugin => format!(
            "Claude Code cannot install plugin \"{name}\". {INSTALL_RULE} Change this entry's \"name\"."
        ),
    }
}

/// What a marketplace or entry `name` breaks before it can be part of an id: a space, a control or
/// bidirectional-formatting character, for a marketplace a path separator or `..`, the grammar. Each rule that fires is reported (the docs list
/// them as separate rows), on `path`. An empty name is left to the reader's own "must have a name".
#[must_use]
pub fn plugin_id_issues(part: IdPart, name: &str, path: &str) -> Vec<PluginIssue> {
    if name.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<PluginIssue> = Vec::new();
    if name.contains(' ') {
        found.push(error(
            path,
            &format!(
                "{} name cannot contain spaces. Use kebab-case (e.g., \"{}\")",
                label(part),
                kebab_example(part)
            ),
        ));
    }
    if contains_control_or_bidi(name) {
        found.push(error(
            path,
            &format!(
                "{} name cannot contain control or bidirectional-formatting characters",
                label(part)
            ),
        ));
    }
    if part == IdPart::Marketplace && (has_marketplace_path(name) || name == ".") {
        found.push(error(path, MARKETPLACE_PATH_MESSAGE));
    }
    if !is_plugin_id_part(name) {
        found.push(error(path, &install_message(part, name)));
    }
    found
}

const DESKTOP_MAX_LENGTH: usize = 128;

/// What the oracle adds about the sync (docs/SPEC.md, "Sources of truth"; spec/judgments.md).
const fn desktop_drops(part: IdPart) -> &'static str {
    match part {
        IdPart::Marketplace => {
            "The Claude Desktop managed marketplace sync (allowedPluginMarketplaces) will reject the whole marketplace."
        }
        IdPart::Plugin => {
            "The Claude Desktop managed marketplace sync (allowedPluginMarketplaces) will drop this entry."
        }
    }
}

/// Claude Desktop's managed marketplace sync rejects these "in any casing" (troubleshooting.md).
const DESKTOP_RESERVED: [&str; 3] = ["org", "org-provisioned", "unknown"];

/// Warnings for names Claude Desktop rejects: the reserved marketplace names and the 128-character grammar.
#[must_use]
pub fn desktop_name_issues(part: IdPart, name: &str, path: &str) -> Vec<PluginIssue> {
    if name.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<PluginIssue> = Vec::new();
    if part == IdPart::Marketplace && DESKTOP_RESERVED.contains(&name.to_lowercase().as_str()) {
        found.push(warning(
            path,
            &format!(
                "Marketplace name \"{name}\" is reserved in Claude Desktop. Claude Code accepts it, but the Claude Desktop managed marketplace sync (allowedPluginMarketplaces) will reject the whole marketplace."
            ),
        ));
    }
    if !is_plugin_id_part(name) || code_unit_length(name) > DESKTOP_MAX_LENGTH {
        found.push(warning(
            path,
            &format!(
                "{} name \"{name}\" is not accepted by Claude Desktop (letters, digits, \".\", \"_\", \"-\"; must start alphanumeric; max {DESKTOP_MAX_LENGTH} chars). {}",
                label(part),
                desktop_drops(part)
            ),
        ));
    }
    found
}

/// A `<plugin>` reference: `name`, or `name@marketplace` (plugins/cli-reference.md, "claude plugin commands").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRef {
    /// The plugin's name.
    pub name: String,
    /// Absent for a bare name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<String>,
}

/// `name` or `name@marketplace`; any other number of `@`, or an empty part, is no reference.
#[must_use]
pub fn parse_plugin_ref(text: &str) -> Option<PluginRef> {
    let parts: Vec<&str> = text.split('@').collect();
    let name = parts.first().copied()?;
    if name.is_empty() || parts.len() > 2 {
        return None;
    }
    if parts.len() == 1 {
        return Some(PluginRef {
            name: name.to_owned(),
            marketplace: None,
        });
    }
    match parts.get(1).copied() {
        None | Some("") => None,
        Some(marketplace) => Some(PluginRef {
            name: name.to_owned(),
            marketplace: Some(marketplace.to_owned()),
        }),
    }
}

/// The name part of a plugin id (`formatter` of `formatter@my-marketplace`); a bare name is its own.
#[must_use]
pub fn plugin_name_of(id: &str) -> String {
    id.split('@').next().unwrap_or(id).to_owned()
}

crate::registry::exports! {
    "format/marketplace/ids" {
        "PLUGIN_ID_PART" => ok || crate::registry::RegExp { source: "^[A-Za-z0-9][A-Za-z0-9._-]*$".to_owned(), flags: String::new() };
        "isPluginIdPart" => ok |name: String| is_plugin_id_part(&name);
        "INSTALL_RULE" => ok || INSTALL_RULE;
        "pluginIdIssues" => ok |part: IdPart, name: String, path: String| plugin_id_issues(part, &name, &path);
        "desktopNameIssues" => ok |part: IdPart, name: String, path: String| desktop_name_issues(part, &name, &path);
        "parsePluginRef" => ok |text: String| parse_plugin_ref(&text);
        "pluginNameOf" => ok |id: String| plugin_name_of(&id);
    }
}
