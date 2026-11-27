//! Port of `src/hooks/output/permission-update.ts`, switched.
//!
//! Permission update entries (hooks.md, "Permission update entries"): the array `updatedPermissions`
//! returns and `permission_suggestions` carries, six entry types, each with a `destination`. The
//! kit validates and normalizes them; writing them to a settings file is the host's.
//!
//! An entry is held as the value the schema answered (the official `PermissionUpdate`), its
//! members in the schema's order, as the TypeScript returns it.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::hooks::input::facts_shared::optional;
use crate::js::{JsValue, string_of};
use crate::protocol::permission_modes::{PERMISSION_BEHAVIORS, normalize_permission_mode};
use crate::zod::{self, Schema, fields, issue};

/// "`session`, `localSettings`, `projectSettings`, `userSettings`". Judgment (spec/judgments.md):
/// the official type also has `cliArg`, which a suggestion can carry and a hook may echo back, so
/// the entry is accepted.
static DESTINATION: LazyLock<Schema> = LazyLock::new(|| {
    zod::r#enum([
        "session",
        "localSettings",
        "projectSettings",
        "userSettings",
        "cliArg",
    ])
});

/// "`rules` is an array of `{toolName, ruleContent?}` objects. Omit `ruleContent` to match the whole tool."
static RULE: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! { "toolName" => zod::string(), "ruleContent" => optional(&zod::string()) })
});
static RULES: LazyLock<Schema> = LazyLock::new(|| zod::array(RULE.clone()));
static BEHAVIOR: LazyLock<Schema> =
    LazyLock::new(|| zod::r#enum(PERMISSION_BEHAVIORS.iter().copied()));
static DIRECTORIES: LazyLock<Schema> = LazyLock::new(|| zod::array(zod::string()));

/// "`manual` as an alias for `default`": the entry carries the mode Claude Code names.
static MODE: LazyLock<Schema> = LazyLock::new(|| {
    zod::string().transform(|raw, context| {
        let raw = raw.as_str().unwrap_or_default();
        if let Some(named) = normalize_permission_mode(raw) {
            return JsValue::from(named);
        }
        context.add_issue(issue("custom").message(format!("\"{raw}\" is not a permission mode")));
        zod::NEVER
    })
});

fn rule_entry(r#type: &str) -> Schema {
    zod::object(fields! {
        "type" => zod::literal(r#type),
        "rules" => RULES.clone(),
        "behavior" => BEHAVIOR.clone(),
        "destination" => DESTINATION.clone(),
    })
}

/// The official `PermissionUpdate`, one of its six entry types.
pub static PERMISSION_UPDATE_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::discriminated_union(
        "type",
        [
            rule_entry("addRules"),
            rule_entry("replaceRules"),
            rule_entry("removeRules"),
            zod::object(fields! {
                "type" => zod::literal("setMode"),
                "mode" => MODE.clone(),
                "destination" => DESTINATION.clone(),
            }),
            zod::object(fields! {
                "type" => zod::literal("addDirectories"),
                "directories" => DIRECTORIES.clone(),
                "destination" => DESTINATION.clone(),
            }),
            zod::object(fields! {
                "type" => zod::literal("removeDirectories"),
                "directories" => DIRECTORIES.clone(),
                "destination" => DESTINATION.clone(),
            }),
        ],
    )
});

/// The valid entries of a value and an issue for each entry that is not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParsedPermissionUpdates {
    /// The official `PermissionUpdate`s, as the schema answered them.
    pub updates: Vec<JsValue>,
    /// One error per entry that is not a valid permission update, `permissionUpdates[i].<field>`.
    pub issues: Vec<PluginIssue>,
}

fn path_of(index: usize, segments: &[JsValue]) -> String {
    segments
        .iter()
        .fold(format!("permissionUpdates[{index}]"), |path, segment| {
            format!("{path}.{}", string_of(segment))
        })
}

/// The valid entries of `raw` and an issue for each that is not: an entry never takes its neighbours down.
#[must_use]
pub fn parse_permission_updates(raw: &JsValue) -> ParsedPermissionUpdates {
    let Some(entries) = raw.as_array() else {
        return ParsedPermissionUpdates {
            updates: Vec::new(),
            issues: vec![error("permissionUpdates", "Expected an array of entries")],
        };
    };
    let mut updates: Vec<JsValue> = Vec::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        match PERMISSION_UPDATE_SCHEMA.safe_parse(entry) {
            Ok(data) => updates.push(data),
            Err(parsed) => issues.extend(
                parsed
                    .issues
                    .iter()
                    .map(|found| error(&path_of(index, found.path()), found.message())),
            ),
        }
    }
    ParsedPermissionUpdates { updates, issues }
}

crate::registry::exports! {
    "hooks/output/permission-update" {
        "parsePermissionUpdates" => ok |raw: JsValue| parse_permission_updates(&raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    fn parsed(text: &str) -> Option<String> {
        let raw = json_parse(text).ok()?;
        serde_json::to_string(&parse_permission_updates(&raw)).ok()
    }

    // The TypeScript before the switch, `parsePermissionUpdates(raw)` printed with `JSON.stringify`.
    #[test]
    fn entries_are_read_as_the_typescript_read_them() {
        let cases = [
            (
                "{}",
                r#"{"updates":[],"issues":[{"severity":"error","path":"permissionUpdates","message":"Expected an array of entries"}]}"#,
            ),
            (
                r#"[{"type":"setMode","mode":"plan","destination":"localSettings"},{"type":"setMode","mode":"x","destination":"projectSettings"}]"#,
                r#"{"updates":[{"type":"setMode","mode":"plan","destination":"localSettings"}],"issues":[{"severity":"error","path":"permissionUpdates[1].mode","message":"\"x\" is not a permission mode"}]}"#,
            ),
            (
                r#"[{"type":"replaceRules","rules":[{"toolName":1}],"behavior":"nope","destination":"session"}]"#,
                r#"{"updates":[],"issues":[{"severity":"error","path":"permissionUpdates[0].rules.0.toolName","message":"Invalid input: expected string, received number"},{"severity":"error","path":"permissionUpdates[0].behavior","message":"Invalid option: expected one of \"allow\"|\"deny\"|\"ask\""}]}"#,
            ),
            (
                r#"[{"type":"removeDirectories","directories":["/a"],"destination":"userSettings","extra":1}]"#,
                r#"{"updates":[{"type":"removeDirectories","directories":["/a"],"destination":"userSettings"}],"issues":[]}"#,
            ),
        ];
        for (raw, want) in cases {
            assert_eq!(parsed(raw).as_deref(), Some(want), "{raw}");
        }
    }
}
