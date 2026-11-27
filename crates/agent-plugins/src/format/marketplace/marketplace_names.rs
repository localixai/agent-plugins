//! Port of `src/format/marketplace/marketplace-names.ts`, switched.
//!
//! Marketplace names (plugins/marketplace-reference.md, "Reserved names" and "Validation messages";
//! errors.md, "Plugin errors"; plugins/troubleshooting.md, "Cannot add marketplace"). The grammar
//! and the control-character rule are `ids.ts`; this file holds the lists the docs print and the
//! checks they describe.
//!
//! The docs split the rules in two. `claude plugin validate` reports impersonation and the reserved
//! names of the "Validation messages" table; "the exact official marketplace names ... pass
//! validation" and are refused when the marketplace is added or loaded, as are the other spellings,
//! the JavaScript member names, the `claudeai-` prefix and the download-folder clash. So there are
//! two functions: `validationNameIssues` (what `validate` prints) and `loadNameIssues` (what adding
//! or loading refuses, which `marketplaceNameIssues` adds to the first).
//!
//! A `MarketplaceSource` is a `JsValue` until `marketplace/types.ts` is in the core.

use std::sync::LazyLock;

use indexmap::IndexSet;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::marketplace_name_lists::{
    ANY_CASING_NAMES, COMMUNITY_NAMES, DIRECTORY_NAMES, NON_MARKETPLACE_NAMES, OBJECT_MEMBER_NAMES,
    OFFICIAL_LISTS, OFFICIAL_NAMES,
};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, Pattern, regex_test};

const PATH: &str = "name";
const BRANDS: [&str; 3] = ["claude", "anthropic", "anthropics"];
/// A brand word followed by one of these reads as the name of an official marketplace (judgment).
const OFFICIAL_SECOND_WORDS: [&str; 6] = [
    "marketplace",
    "marketplaces",
    "plugin",
    "plugins",
    "official",
    "directory",
];

/// The name as the impersonation check reads it: lower case, every run of characters that is not a
/// letter, digit or underscore one hyphen. An underscore is not a separator because the docs name
/// "a symbol other than an underscore" as the one that spells a hyphen (judgment, spec/judgments.md).
fn words_of(name: &str) -> Vec<String> {
    let mut hyphenated = String::new();
    let mut in_run = false;
    for character in name.to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_' {
            hyphenated.push(character);
            in_run = false;
        } else if !in_run {
            hyphenated.push('-');
            in_run = true;
        }
    }
    hyphenated
        .trim_matches('-')
        .split('-')
        .map(str::to_owned)
        .collect()
}

/// A name that starts with the words of an official name that begins with a brand word.
static OFFICIAL_BRAND_PREFIXES: LazyLock<Vec<Vec<&'static str>>> = LazyLock::new(|| {
    OFFICIAL_NAMES
        .iter()
        .chain(COMMUNITY_NAMES.iter())
        .chain(DIRECTORY_NAMES.iter())
        .map(|name| name.split('-').collect::<Vec<&str>>())
        .filter(|words| BRANDS.contains(&words.first().copied().unwrap_or("")))
        .collect()
});

fn starts_with_words(words: &[String], prefix: &[&str]) -> bool {
    prefix
        .iter()
        .enumerate()
        .all(|(index, word)| words.get(index).is_some_and(|one| one == word))
}

fn word_at(words: &[String], index: Option<usize>) -> &str {
    index
        .and_then(|at| words.get(at))
        .map_or("", String::as_str)
}

fn beside_official(words: &[String]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        word == "official"
            && (BRANDS.contains(&word_at(words, index.checked_sub(1)))
                || BRANDS.contains(&word_at(words, Some(index + 1))))
    })
}

fn leads_with_brand(words: &[String]) -> bool {
    BRANDS.contains(&word_at(words, Some(0)))
        && (OFFICIAL_SECOND_WORDS.contains(&word_at(words, Some(1)))
            || OFFICIAL_BRAND_PREFIXES
                .iter()
                .any(|prefix| starts_with_words(words, prefix)))
}

/// Which reserved name a trailing dot or a symbol in place of a hyphen spells ("v2.1.280"), if any.
static RESERVED_SPELLINGS: LazyLock<IndexSet<&'static str>> = LazyLock::new(|| {
    OFFICIAL_LISTS
        .iter()
        .copied()
        .chain(NON_MARKETPLACE_NAMES.keys().copied())
        .collect()
});

/// The reserved name a spelling of `name` is, if any.
#[must_use]
pub fn spelling_of(name: &str) -> Option<String> {
    // `/[^A-Za-z0-9_-]/g` is a character class over UTF-16 code units: a character above U+FFFF
    // is two of them, so it becomes two hyphens.
    let spelled: String = name
        .trim_end_matches('.')
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character.to_string()
            } else {
                "-".repeat(character.len_utf16())
            }
        })
        .collect();
    (spelled != name && RESERVED_SPELLINGS.contains(spelled.as_str())).then_some(spelled)
}

/// Names such as `official-claude-plugins` or `claude-plugins-v2`, and any non-ASCII name (even one that spells a reserved name).
#[must_use]
pub fn impersonates(name: &str) -> bool {
    if !name.is_ascii() {
        return true;
    }
    if RESERVED_SPELLINGS.contains(name) || spelling_of(name).is_some() {
        return false;
    }
    let words = words_of(name);
    beside_official(&words) || leads_with_brand(&words)
}

fn reserved_issue(name: &str) -> PluginIssue {
    error(PATH, &format!("Marketplace name \"{name}\" is reserved"))
}

fn reserved_variant_issues(name: &str) -> Vec<PluginIssue> {
    if let Some(reason) = NON_MARKETPLACE_NAMES.get(name) {
        return vec![error(
            PATH,
            &format!("Marketplace name \"{name}\" {reason}"),
        )];
    }
    if ANY_CASING_NAMES.contains(name) {
        vec![reserved_issue(name)]
    } else {
        Vec::new()
    }
}

/// The six package-manager names are reserved in any letter case, but a validator reports only the
/// lower case; adding or loading refuses the other casings (spec J15.3: the 2.1.321 table for the
/// verdict, oracle 2.1.325 passes `NPM`).
fn other_casing_issues(name: &str) -> Vec<PluginIssue> {
    if name != name.to_lowercase() && ANY_CASING_NAMES.contains(name.to_lowercase().as_str()) {
        vec![reserved_issue(name)]
    } else {
        Vec::new()
    }
}

/// What `claude plugin validate` reports about a marketplace's name beyond the id grammar.
#[must_use]
pub fn validation_name_issues(name: &str) -> Vec<PluginIssue> {
    let mut found = reserved_variant_issues(name);
    if impersonates(name) {
        found.push(error(
            PATH,
            "Marketplace name impersonates an official Anthropic/Claude marketplace",
        ));
    }
    found
}

/// `true` for the origins whose names may be the official ones: a `github` or `git` source under
/// `github.com/anthropics/`. JavaScript's `i` without `u` folds ASCII letters only (RS6).
static ANTHROPICS_URL: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^(?i-u:(?:https?://github\.com/|ssh://git@github\.com/|git@github\.com:)anthropics/)[^/]",
    )
    .ok()
});

/// The address a marketplace source is hosted at, for the kinds that have one.
fn origin_url(origin: &JsValue) -> Option<String> {
    match origin.get("kind").and_then(JsValue::as_str)? {
        "github" => origin
            .get("repo")
            .and_then(JsValue::as_str)
            .map(|repo| format!("https://github.com/{repo}")),
        "git" => origin
            .get("url")
            .and_then(JsValue::as_str)
            .map(str::to_owned),
        "url" | "npm" | "file" | "directory" | "settings" | "skills-dir" | "hostPattern"
        | "pathPattern" => None,
        _ => None,
    }
}

/// Whether the source is a git repository on GitHub (the kinds the refusal text tells apart).
fn is_git_host(origin: &JsValue) -> bool {
    matches!(
        origin.get("kind").and_then(JsValue::as_str),
        Some("github" | "git")
    )
}

fn from_anthropics(origin: &JsValue) -> bool {
    origin_url(origin).is_some_and(|url| regex_test(&ANTHROPICS_URL, &url))
}

fn official_name_issues(name: &str, origin: &JsValue) -> Vec<PluginIssue> {
    if !OFFICIAL_LISTS.contains(name) || from_anthropics(origin) {
        return Vec::new();
    }
    let reserved = format!("The name '{name}' is reserved for official Anthropic marketplaces.");
    let message = if is_git_host(origin) {
        format!("{reserved} Only repositories from 'github.com/anthropics/' can use this name.")
    } else {
        format!(
            "{reserved} The name can only be used with GitHub sources from the 'anthropics' organization."
        )
    };
    vec![error(PATH, &message)]
}

fn spelling_issues(name: &str) -> Vec<PluginIssue> {
    spelling_of(name).map_or_else(Vec::new, |reserved| {
        vec![error(
            PATH,
            &format!(
                "\"{name}\" is another spelling of \"{reserved}\", a reserved marketplace name"
            ),
        )]
    })
}

fn member_issues(name: &str) -> Vec<PluginIssue> {
    if !OBJECT_MEMBER_NAMES.contains(name) {
        return Vec::new();
    }
    vec![error(
        PATH,
        &format!(
            "Cannot add marketplace \"{name}\": Claude Code reserves this name and cannot register a marketplace under it. The name is set by \"name\" in the marketplace's marketplace.json; ask its maintainer to change it."
        ),
    )]
}

const CLAUDE_AI_PREFIX: &str = "claudeai-";

fn claude_ai_issues(name: &str, hosted_on_claude_ai: bool) -> Vec<PluginIssue> {
    if !name.starts_with(CLAUDE_AI_PREFIX) || hosted_on_claude_ai {
        return Vec::new();
    }
    vec![error(
        PATH,
        &format!(
            "Cannot add marketplace \"{name}\": names starting with \"{CLAUDE_AI_PREFIX}\" are reserved for marketplaces hosted on claude.ai"
        ),
    )]
}

/// Where a marketplace came from.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameOrigin {
    /// Where the marketplace came from (a `MarketplaceSource`); only a `github` or `git` source under `github.com/anthropics/` may use an official name.
    #[serde(default, skip_serializing_if = "JsValue::is_undefined")]
    pub origin: JsValue,
    /// The marketplace is hosted on claude.ai, the only one that may start with `claudeai-`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_on_claude_ai: Option<bool>,
}

/// What adding or loading a marketplace refuses about its name, on top of `validationNameIssues`.
#[must_use]
pub fn load_name_issues(name: &str, origin: &NameOrigin) -> Vec<PluginIssue> {
    [
        other_casing_issues(name),
        official_name_issues(name, &origin.origin),
        spelling_issues(name),
        member_issues(name),
        claude_ai_issues(name, origin.hosted_on_claude_ai == Some(true)),
    ]
    .concat()
}

/// Every refusal a marketplace's name can draw: `validate`'s and adding or loading's.
#[must_use]
pub fn marketplace_name_issues(name: &str, origin: &NameOrigin) -> Vec<PluginIssue> {
    [validation_name_issues(name), load_name_issues(name, origin)].concat()
}

/// A registered marketplace, as far as the download-folder rule needs it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegisteredMarketplace {
    /// The name it is registered under.
    pub name: String,
    /// Where it comes from (a `MarketplaceSource`).
    pub source: JsValue,
}

/// Only a `github` source downloads through an `<owner>-<repo>` folder.
fn is_github(source: &JsValue) -> bool {
    source.get("kind").and_then(JsValue::as_str) == Some("github")
}

/// The folder Claude Code downloads a `github` marketplace through: `acme/x-tools` is `acme-x-tools`.
fn download_folder(source: &JsValue) -> Option<String> {
    source
        .get("repo")
        .and_then(JsValue::as_str)
        .map(|repo| repo.replacen('/', "-", 1))
}

/// Adding a marketplace named like the download folder of another registered GitHub marketplace is
/// refused while that marketplace is registered under a different name (requires Claude Code
/// v2.1.290 or later).
#[must_use]
pub fn download_folder_issues(
    name: &str,
    registered: &[RegisteredMarketplace],
) -> Vec<PluginIssue> {
    let owner = registered.iter().find(|other| {
        other.name != name
            && is_github(&other.source)
            && download_folder(&other.source).as_deref() == Some(name)
    });
    owner.map_or_else(Vec::new, |owner| {
        vec![error(
            PATH,
            &format!(
                "Can't use the marketplace name \"{name}\": it is the download folder of the registered marketplace \"{}\"",
                owner.name
            ),
        )]
    })
}

crate::registry::exports! {
    "format/marketplace/marketplace-names" {
        "spellingOf" => ok |name: String| spelling_of(&name);
        "validationNameIssues" => ok |name: String| validation_name_issues(&name);
        "loadNameIssues" => ok |name: String, origin: Option<NameOrigin>| load_name_issues(&name, &origin.unwrap_or_default());
        "marketplaceNameIssues" => ok |name: String, origin: Option<NameOrigin>| marketplace_name_issues(&name, &origin.unwrap_or_default());
        "downloadFolderIssues" => ok |name: String, registered: Vec<RegisteredMarketplace>| download_folder_issues(&name, &registered);
    }
}
