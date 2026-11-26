//! Port of `src/convert/profiles.ts`, switched.
//!
//! The tool matchers that select exactly one core tool profile (Agent Plugins Extended §18.3.4,
//! §18.4.3 item 4, the table of §18.2.7): a Claude Code matcher converts when it names one profile's
//! tools, and a core handler that a converted adapter limits to one profile converts back to that
//! matcher.

use serde::{Deserialize, Serialize};

/// A core tool profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileName {
    /// `shell`.
    #[serde(rename = "shell")]
    Shell,
    /// `file.read`.
    #[serde(rename = "file.read")]
    FileRead,
    /// `file.modify`.
    #[serde(rename = "file.modify")]
    FileModify,
    /// `web.fetch`.
    #[serde(rename = "web.fetch")]
    WebFetch,
    /// `web.search`.
    #[serde(rename = "web.search")]
    WebSearch,
    /// `mcp`.
    #[serde(rename = "mcp")]
    Mcp,
}

/// The profiles, in the order of the table.
pub const PROFILE_NAMES: [ProfileName; 6] = [
    ProfileName::Shell,
    ProfileName::FileRead,
    ProfileName::FileModify,
    ProfileName::WebFetch,
    ProfileName::WebSearch,
    ProfileName::Mcp,
];

/// Each profile's Claude Code tools, as the matcher of §18.3.4 lists them.
const fn tools_of(profile: ProfileName) -> &'static [&'static str] {
    match profile {
        ProfileName::Shell => &["Bash", "PowerShell"],
        ProfileName::FileRead => &["Read"],
        ProfileName::FileModify => &["Edit", "Write", "NotebookEdit"],
        ProfileName::WebFetch => &["WebFetch"],
        ProfileName::WebSearch => &["WebSearch"],
        ProfileName::Mcp => &["mcp__.*"],
    }
}

/// The text of a profile's name.
const fn name_of(profile: ProfileName) -> &'static str {
    match profile {
        ProfileName::Shell => "shell",
        ProfileName::FileRead => "file.read",
        ProfileName::FileModify => "file.modify",
        ProfileName::WebFetch => "web.fetch",
        ProfileName::WebSearch => "web.search",
        ProfileName::Mcp => "mcp",
    }
}

/// Whether a text names a profile.
#[must_use]
pub fn is_profile_name(value: Option<&str>) -> bool {
    PROFILE_NAMES
        .iter()
        .any(|name| Some(name_of(*name)) == value)
}

/// The matcher that selects a profile's tools.
#[must_use]
pub fn matcher_of(profile: ProfileName) -> String {
    tools_of(profile).join("|")
}

/// A matcher that matches every tool (hooks.md, "Matcher patterns": omitted, `""` or `*`).
#[must_use]
pub fn matches_everything(matcher: Option<&str>) -> bool {
    matches!(matcher, None | Some("" | "*"))
}

fn same_set(first: &[&str], second: &[&str]) -> bool {
    first.len() == second.len() && first.iter().all(|name| second.contains(name))
}

/// The one profile a matcher selects, in any order of its names; `None` for any other matcher.
#[must_use]
pub fn profile_of_matcher(matcher: &str) -> Option<ProfileName> {
    let names: Vec<&str> = matcher.split('|').collect();
    PROFILE_NAMES
        .iter()
        .find(|profile| same_set(tools_of(**profile), &names))
        .copied()
}

crate::registry::exports! {
    "convert/profiles" {
        "PROFILE_NAMES" => ok || PROFILE_NAMES;
        "isProfileName" => ok |value: Option<String>| is_profile_name(value.as_deref());
        "matcherOf" => ok |profile: ProfileName| matcher_of(profile);
        "matchesEverything" => ok |matcher: Option<String>| matches_everything(matcher.as_deref());
        "profileOfMatcher" => ok |matcher: String| profile_of_matcher(&matcher);
    }
}
