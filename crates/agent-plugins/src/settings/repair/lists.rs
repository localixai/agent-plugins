//! Port of `src/settings/repair/lists.ts`, switched.
//!
//! The repairs of the list values: texts, the allowlists that fail closed, the channel plugins and
//! the marketplace sources.

use super::support::{RepairAt, Repaired, dropped, failed_closed, kept, strip_invalid_now, where_};
use crate::format::issues::warning;
use crate::js::regexp::RegExp;
use crate::js::{JsObject, JsValue, is_whitespace};
use crate::protocol::is_record::is_record;

fn read_string(entry: &JsValue) -> Option<JsValue> {
    entry.is_string().then(|| entry.clone())
}

/// A list of texts: a non-text entry is dropped, a value that is no list is dropped whole.
#[must_use]
pub fn string_list(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return dropped(at, &format!("\"{}\" must be a list of texts", at.key));
    };
    let stripped = strip_invalid_now(items, at, read_string, "Expected a text");
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// `allowedHttpHookUrls` and `httpHookAllowedEnvVars`: an allowlist that is unreadable in managed
/// settings is enforced as an empty one (managed-settings.md, "Keys that fail closed"), an entry
/// that is invalid is stripped and the rest enforced.
#[must_use]
pub fn closed_string_list(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_array() {
        return string_list(value, at);
    }
    if at.closed {
        failed_closed(at, JsValue::Array(Vec::new()), "an empty allowlist")
    } else {
        string_list(value, at)
    }
}

/// `plugin@marketplace`, the string form of an `allowedChannelPlugins` entry
/// (settings-reference.md): `/^([^@\s]+)@([^@\s]+)$/`.
fn channel_string(entry: &str) -> Option<(&str, &str)> {
    let part = |text: &str| !text.is_empty() && !text.chars().any(is_whitespace);
    entry.split_once('@').filter(|(plugin, marketplace)| {
        part(plugin) && part(marketplace) && !marketplace.contains('@')
    })
}

fn channel_ref(marketplace: &str, plugin: &str) -> JsValue {
    JsValue::Object(JsObject::from([
        (
            "marketplace".to_owned(),
            JsValue::String(marketplace.to_owned()),
        ),
        ("plugin".to_owned(), JsValue::String(plugin.to_owned())),
    ]))
}

fn read_channel_entry(entry: &JsValue) -> Option<JsValue> {
    if let Some(text) = entry.as_str() {
        return channel_string(text).map(|(plugin, marketplace)| channel_ref(marketplace, plugin));
    }
    if !is_record(entry) {
        return None;
    }
    let marketplace = entry.get("marketplace").and_then(JsValue::as_str)?;
    let plugin = entry.get("plugin").and_then(JsValue::as_str)?;
    Some(channel_ref(marketplace, plugin))
}

/// `allowedChannelPlugins`: an entry is `{ marketplace, plugin }` or the string
/// `plugin@marketplace`, which Claude Code "treats as the equivalent object", so the result holds
/// objects only.
#[must_use]
pub fn channel_allowlist(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return if at.closed {
            failed_closed(
                at,
                JsValue::Array(Vec::new()),
                "an empty allowlist: no channel plugin is admitted",
            )
        } else {
            dropped(at, &format!("\"{}\" must be a list", at.key))
        };
    };
    let stripped = strip_invalid_now(
        items,
        at,
        read_channel_entry,
        "Expected { marketplace, plugin } or \"plugin@marketplace\"",
    );
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// The text fields each source kind requires (settings-reference.md, "Allowed source types").
fn required_fields(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "github" => Some(&["repo"]),
        "git" | "url" => Some(&["url"]),
        "file" | "directory" => Some(&["path"]),
        "hostPattern" => Some(&["hostPattern"]),
        "pathPattern" => Some(&["pathPattern"]),
        "skills-dir" => Some(&[]),
        // The docs' table has no row for these two; the SDK types them (spec/judgments.md).
        "npm" => Some(&["package"]),
        "settings" => Some(&["name"]),
        _ => None,
    }
}

/// The fields of a source that are regular expressions, compiled with JavaScript's `RegExp`.
const REGEX_FIELDS: [&str; 2] = ["hostPattern", "pathPattern"];

/// A source object: a known `source` kind with the text fields that kind requires.
fn read_source(entry: &JsValue) -> Option<JsValue> {
    if !is_record(entry) {
        return None;
    }
    let fields = required_fields(entry.get("source").and_then(JsValue::as_str)?)?;
    fields
        .iter()
        .all(|field| entry.get(field).is_some_and(JsValue::is_string))
        .then(|| entry.clone())
}

/// The regex fields of a source entry that do not compile.
fn broken_regex_fields(entry: &JsValue) -> Vec<&'static str> {
    REGEX_FIELDS
        .into_iter()
        .filter(|field| {
            entry
                .get(field)
                .and_then(JsValue::as_str)
                .is_some_and(|pattern| RegExp::new(pattern, "").is_err())
        })
        .collect()
}

/// `strictKnownMarketplaces`: unreadable in managed settings it is an empty allowlist, a total
/// lockdown; an entry that is invalid or "can't be enforced, such as a `hostPattern` regex that
/// doesn't compile, is stripped and the valid subset is enforced" (managed-settings.md).
#[must_use]
pub fn strict_sources(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return if at.closed {
            failed_closed(
                at,
                JsValue::Array(Vec::new()),
                "an empty allowlist: no marketplace source is admitted",
            )
        } else {
            dropped(
                at,
                &format!("\"{}\" must be a list of marketplace sources", at.key),
            )
        };
    };
    let stripped = strip_invalid_now(
        items,
        at,
        |entry| read_source(entry).filter(|source| broken_regex_fields(source).is_empty()),
        "Not a marketplace source the allowlist can enforce",
    );
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// `blockedMarketplaces`: an invalid entry is stripped; "an entry that parses but can never
/// match, such as a `hostPattern` regex that doesn't compile, is kept with a warning" (so the
/// restriction stays active); a wholly invalid value is dropped with a warning, since blocking
/// every marketplace would block sources the policy never named (managed-settings.md).
#[must_use]
pub fn blocked_sources(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return dropped(
            at,
            &format!("\"{}\" must be a list of marketplace sources", at.key),
        );
    };
    let stripped = strip_invalid_now(items, at, read_source, "Not a marketplace source");
    let mut issues = stripped.issues;
    for (index, entry) in stripped.entries.iter().enumerate() {
        for field in broken_regex_fields(entry) {
            issues.push(warning(
                &where_(at, &format!("[{index}]")),
                &format!(
                    "\"{field}\" is not a valid regular expression, so the entry blocks nothing"
                ),
            ));
        }
    }
    kept(JsValue::Array(stripped.entries), issues)
}

crate::registry::exports! {
    "settings/repair/lists" {
        "stringList" => ok |value: JsValue, at: RepairAt| string_list(&value, &at);
        "closedStringList" => ok |value: JsValue, at: RepairAt| closed_string_list(&value, &at);
        "channelAllowlist" => ok |value: JsValue, at: RepairAt| channel_allowlist(&value, &at);
        "strictSources" => ok |value: JsValue, at: RepairAt| strict_sources(&value, &at);
        "blockedSources" => ok |value: JsValue, at: RepairAt| blocked_sources(&value, &at);
    }
}

#[cfg(test)]
mod tests {
    use super::channel_string;

    #[test]
    fn a_channel_string_is_plugin_at_marketplace() {
        assert_eq!(channel_string("p@m"), Some(("p", "m")));
        assert_eq!(channel_string("p@m@x"), None);
        assert_eq!(channel_string("p @m"), None);
    }
}
