//! Ported from `src/settings/marketplaces/headers-helper.ts`; only the core uses it now, so it has no wrapper.
//!
//! What the planned source of an `extraKnownMarketplaces` entry keeps of its request headers and
//! `headersHelper` commands (settings-reference.md, `extraKnownMarketplaces`; plugins/host-marketplace.md,
//! "When Claude Code skips a headersHelper command or drops its output"). Running a command is the host's.
//!
//! The package's tables of per-kind functions (`ByKind`) are `match`es over [`SourceKind`] here, in
//! the same order. A `url` whose `url` is not a string, which the TypeScript's `startsWith` would
//! throw on, reads as not `https://`: such an entry is invalid anyway (`source-validate.ts`).

use crate::format::marketplace::header_names::is_routing_header;
use crate::js::{JsObject, JsValue};
use crate::protocol::is_record::is_record;
use crate::protocol::settings_keys::MARKETPLACE_SOURCE_KINDS;
use crate::settings::layers::PolicyScope;
use crate::settings::marketplaces::source_kinds::{SourceKind, by_kind};

/// The source with every member but `headersHelper` (`({ headersHelper: _helper, ...rest }) => rest`).
fn without_helper_member(source: &JsValue) -> JsValue {
    match source.as_object() {
        Some(members) => JsValue::Object(
            members
                .iter()
                .filter(|(key, _)| key.as_str() != "headersHelper")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        None => source.clone(),
    }
}

/// Applies a change to each inline plugin entry of a `settings` source. A plugin entry that is not an
/// object is left as it is; the entry is invalid anyway (`source-validate.ts`). No `plugins` stays
/// none: the boundary drops the `undefined` member the spread writes.
fn on_plugins(change: impl Fn(&JsValue) -> JsValue, source: &JsValue) -> JsValue {
    let mut members: JsObject = source.as_object().cloned().unwrap_or_default();
    let plugins = match source.get("plugins") {
        Some(JsValue::Array(plugins)) => JsValue::Array(
            plugins
                .iter()
                .map(|plugin| {
                    if is_record(plugin) {
                        change(plugin)
                    } else {
                        plugin.clone()
                    }
                })
                .collect(),
        ),
        Some(other) => other.clone(),
        None => return JsValue::Object(members),
    };
    members.insert("plugins".to_owned(), plugins);
    JsValue::Object(members)
}

/// The helper of a `url` source and of each inline plugin entry. The fixed `headers` stay.
fn without_helper(kind: SourceKind, source: &JsValue) -> JsValue {
    match kind {
        SourceKind::Github
        | SourceKind::Git
        | SourceKind::File
        | SourceKind::Directory
        | SourceKind::Npm
        | SourceKind::HostPattern
        | SourceKind::PathPattern
        | SourceKind::SkillsDir => source.clone(),
        SourceKind::Url => without_helper_member(source),
        SourceKind::Settings => on_plugins(without_helper_member, source),
    }
}

/// "Marketplace URL doesn't start with `https://`": that `url` source's command doesn't run.
fn without_insecure_helper(kind: SourceKind, source: &JsValue) -> JsValue {
    match kind {
        SourceKind::Url => {
            if source
                .get("url")
                .and_then(JsValue::as_str)
                .is_some_and(|url| url.starts_with("https://"))
            {
                source.clone()
            } else {
                by_kind(without_helper, source).unwrap_or_else(|| source.clone())
            }
        }
        SourceKind::Github
        | SourceKind::Git
        | SourceKind::File
        | SourceKind::Directory
        | SourceKind::Npm
        | SourceKind::Settings
        | SourceKind::HostPattern
        | SourceKind::PathPattern
        | SourceKind::SkillsDir => source.clone(),
    }
}

/// An inline plugin entry without its routing headers, when its `headers` is an object.
fn plugin_without_routing_headers(plugin: &JsValue) -> JsValue {
    let (Some(members), Some(headers)) = (
        plugin.as_object(),
        plugin.get("headers").and_then(JsValue::as_object),
    ) else {
        return plugin.clone();
    };
    let kept: JsObject = headers
        .iter()
        .filter(|(name, _)| !is_routing_header(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let mut members = members.clone();
    members.insert("headers".to_owned(), JsValue::Object(kept));
    JsValue::Object(members)
}

/// The header filter, which applies to an inline plugin entry's `headers` only.
fn without_routing_headers(kind: SourceKind, source: &JsValue) -> JsValue {
    match kind {
        SourceKind::Settings => on_plugins(plugin_without_routing_headers, source),
        SourceKind::Github
        | SourceKind::Git
        | SourceKind::Url
        | SourceKind::File
        | SourceKind::Directory
        | SourceKind::Npm
        | SourceKind::HostPattern
        | SourceKind::PathPattern
        | SourceKind::SkillsDir => source.clone(),
    }
}

/// `byKind(registry, source)` for a source whose kind is known.
fn apply(registry: fn(SourceKind, &JsValue) -> JsValue, source: JsValue) -> JsValue {
    by_kind(registry, &source).unwrap_or(source)
}

/// What a declaration of that scope loses: a repository can supply a project or local file.
fn declared_in(from: PolicyScope, source: JsValue) -> JsValue {
    match from {
        PolicyScope::Managed | PolicyScope::Parent | PolicyScope::Flag | PolicyScope::User => {
            source
        }
        PolicyScope::Project | PolicyScope::Local => apply(without_routing_headers, source),
        PolicyScope::AddDir => apply(without_routing_headers, apply(without_helper, source)),
    }
}

fn is_known_kind(kind: Option<&str>) -> bool {
    kind.is_some_and(|kind| MARKETPLACE_SOURCE_KINDS.contains(&kind))
}

/// The source a host plans from, for an entry a file of this scope declares. A kind nobody lists is
/// left as it is: the entry is invalid (`source-validate.ts`).
#[must_use]
pub fn planned_source(source: JsValue, from: PolicyScope) -> JsValue {
    if is_known_kind(source.get("source").and_then(JsValue::as_str)) {
        declared_in(from, apply(without_insecure_helper, source))
    } else {
        source
    }
}
