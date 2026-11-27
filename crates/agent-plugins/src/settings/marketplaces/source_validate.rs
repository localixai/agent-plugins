//! Port of `src/settings/marketplaces/source-validate.ts`, switched.
//!
//! What an `extraKnownMarketplaces` entry's `source` must be to register a marketplace
//! (settings-reference.md, `extraKnownMarketplaces` "Marketplace source types";
//! plugins/marketplace-reference.md is the marketplace stream's). One registry over every source
//! kind, so a new kind fails to compile.

use super::source_kinds::{SourceKind, by_kind_with};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, string_of};
use crate::protocol::is_record::is_record;
use crate::protocol::settings_keys::MARKETPLACE_SOURCE_KINDS;

/// `source.source` as a template literal writes it.
fn kind_text(source: &JsValue) -> String {
    string_of(source.get("source").unwrap_or(&JsValue::Undefined))
}

/// The kinds of the policy lists that name no place a marketplace comes from.
fn not_a_place(source: &JsValue) -> Vec<String> {
    vec![format!(
        "source \"{}\" belongs to the allowlist and blocklist and names no place a marketplace comes from",
        kind_text(source)
    )]
}

/// Whether each plugin source kind is "external". The docs say "external sources such as GitHub
/// or npm" and the SDK says a relative path "has no marketplace repository to resolve against";
/// `unsupported` is the placeholder for a source Claude Code cannot read (spec/judgments.md:
/// `command` counts as external). Typed over every kind, so a new SDK kind fails to compile.
const EXTERNAL: &[(&str, bool)] = &[
    ("npm", true),
    ("url", true),
    ("github", true),
    ("git-subdir", true),
    ("archive", true),
    ("command", true),
    ("unsupported", false),
];

fn external_of(kind: &JsValue) -> Option<bool> {
    let kind = kind.as_str()?;
    EXTERNAL
        .iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, external)| *external)
}

fn is_external(plugin: &JsValue) -> bool {
    is_record(plugin)
        && plugin.get("source").is_some_and(is_record)
        && plugin
            .get("source")
            .and_then(|source| source.get("source"))
            .and_then(external_of)
            .unwrap_or(false)
}

fn plugin_name(plugin: &JsValue, index: usize) -> String {
    plugin
        .get("name")
        .and_then(JsValue::as_str)
        .map_or_else(|| format!("#{index}"), ToOwned::to_owned)
}

/// An inline marketplace: its `name` is the key, and each plugin is fetched from elsewhere.
fn inline_problems(source: &JsValue, key: &str) -> Vec<String> {
    let name = source.get("name");
    let mut problems = if name.and_then(JsValue::as_str) == Some(key) {
        Vec::new()
    } else {
        vec![format!(
            "the \"settings\" source name \"{}\" must equal the marketplace key \"{key}\"",
            string_of(name.unwrap_or(&JsValue::Undefined))
        )]
    };
    let plugins = source
        .get("plugins")
        .and_then(JsValue::as_array)
        .map_or(&[][..], Vec::as_slice);
    for (index, plugin) in plugins.iter().enumerate() {
        if !is_external(plugin) {
            problems.push(format!(
                "plugin \"{}\" must reference an external source such as GitHub or npm",
                plugin_name(plugin, index)
            ));
        }
    }
    problems
}

fn problems(kind: SourceKind, source: &JsValue, key: &str) -> Vec<String> {
    match kind {
        SourceKind::Github
        | SourceKind::Git
        | SourceKind::Url
        | SourceKind::File
        | SourceKind::Directory
        | SourceKind::Npm => Vec::new(),
        SourceKind::Settings => inline_problems(source, key),
        SourceKind::HostPattern | SourceKind::PathPattern | SourceKind::SkillsDir => {
            not_a_place(source)
        }
    }
}

fn is_known_kind(kind: Option<&JsValue>) -> bool {
    kind.and_then(JsValue::as_str)
        .is_some_and(|kind| MARKETPLACE_SOURCE_KINDS.contains(&kind))
}

/// Why the entry named `key` cannot register a marketplace; empty when it can. A kind the docs
/// and the SDK do not list is not a marketplace either: the repair of the key only checks for a
/// `source` object.
#[must_use]
pub fn registration_problems(key: &str, source: &JsValue) -> Vec<PluginIssue> {
    let path = format!("extraKnownMarketplaces.{key}.source");
    if !is_known_kind(source.get("source")) {
        return vec![error(
            &path,
            &format!(
                "source \"{}\" is not a marketplace source the docs list",
                kind_text(source)
            ),
        )];
    }
    by_kind_with(
        |kind, source, key: &&str| problems(kind, source, key),
        source,
        &key,
    )
    .unwrap_or_default()
    .iter()
    .map(|message| error(&path, message))
    .collect()
}

crate::registry::exports! {
    "settings/marketplaces/source-validate" {
        "registrationProblems" => ok |key: String, source: JsValue| registration_problems(&key, &source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_settings_source_names_its_key_and_external_plugins() {
        let source = crate::js::json_parse::json_parse(
            r#"{"source":"settings","name":"b","plugins":[{"name":"p","source":{"source":"npm"}},{"source":"./x"}]}"#,
        )
        .unwrap_or_default();
        let messages: Vec<String> = registration_problems("a", &source)
            .into_iter()
            .map(|issue| issue.message)
            .collect();
        assert_eq!(
            messages,
            vec![
                "the \"settings\" source name \"b\" must equal the marketplace key \"a\""
                    .to_owned(),
                "plugin \"#1\" must reference an external source such as GitHub or npm".to_owned(),
            ]
        );
    }
}
