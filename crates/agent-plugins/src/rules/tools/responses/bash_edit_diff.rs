//! Port of `src/rules/tools/responses/bash-edit-diff.ts`, switched.
//!
//! One entry of `bashEditDiff.files` (hooks.md, "Bash"): the diff of a changed file "for display".
//! `hunks` is shown in the table and its shape is not documented, so it is kept as it came and is
//! not required (spec/judgments.md). The SDK's `BashOutput` has no `bashEditDiff` (the docs name
//! Claude Code v2.1.269), so nothing ties these types to the SDK.
//!
//! A diff is held as zod's output, the object the TypeScript returns, and read field by field.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::shape::{RESPONSE_ROOT, issues_of, object_of};
use crate::format::issues::{PluginIssue, error, warning};
use crate::js::{JsValue, Pattern, regex_test};
use crate::protocol::is_record::is_record;
use crate::rules::tools::inputs::field_schemas::{FLAG, NUM, TEXT, list};
use crate::zod::{self, Schema, fields};

/// What reading `bashEditDiff` gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BashEditDiffParse {
    /// `undefined` when the call carries no `bashEditDiff` or it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<JsValue>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// "at most 200" paths and "up to 5" diffs.
const MAX_CHANGED_FILES: usize = 200;
const MAX_DIFFED_FILES: usize = 5;

static FIELD: LazyLock<String> = LazyLock::new(|| format!("{RESPONSE_ROOT}.bashEditDiff"));

static COUNT: LazyLock<Schema> = LazyLock::new(|| {
    NUM.refine(
        |value| {
            value
                .as_f64()
                .is_some_and(|number| number.is_finite() && number.fract() == 0.0 && number >= 0.0)
        },
        "expected a whole number from 0",
    )
});

static SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    object_of(fields! {
        "changedFiles" => list(TEXT.clone()).optional(),
        "files" => list(
            object_of(fields! {
                "filePath" => TEXT.clone(),
                "hunks" => zod::unknown().optional(),
                "created" => FLAG.optional(),
                "deleted" => FLAG.optional(),
            }),
        ).optional(),
        "moreFiles" => COUNT.optional(),
        "unavailable" => FLAG.optional(),
        "skipped" => FLAG.optional(),
        "shared" => FLAG.optional(),
    })
});

/// A POSIX path, a Windows drive path or a UNC path; hooks receive native separators.
static ABSOLUTE: Pattern = LazyLock::new(|| Regex::new(r"^(?:[\\/]|[A-Za-z]:[\\/])").ok());

fn not_absolute(paths: &[&str], at: &str) -> Vec<PluginIssue> {
    paths
        .iter()
        .enumerate()
        .filter(|(_, path)| !regex_test(&ABSOLUTE, path))
        .map(|(index, _)| warning(&format!("{at}.{index}"), "is not an absolute path"))
        .collect()
}

/// A list member of the diff (`[]` when absent, as the TypeScript's defaults give it).
fn items<'a>(diff: &'a JsValue, name: &str) -> Option<&'a Vec<JsValue>> {
    diff.get(name).and_then(JsValue::as_array)
}

fn texts(values: &[JsValue]) -> Vec<&str> {
    values.iter().filter_map(JsValue::as_str).collect()
}

type Check = fn(&JsValue) -> Vec<PluginIssue>;

fn too_many_paths(diff: &JsValue) -> Vec<PluginIssue> {
    match items(diff, "changedFiles") {
        Some(changed) if changed.len() > MAX_CHANGED_FILES => vec![warning(
            &format!("{}.changedFiles", *FIELD),
            &format!(
                "lists {} paths and the docs allow at most {MAX_CHANGED_FILES}",
                changed.len()
            ),
        )],
        _ => Vec::new(),
    }
}

fn too_many_diffs(diff: &JsValue) -> Vec<PluginIssue> {
    match items(diff, "files") {
        Some(files) if files.len() > MAX_DIFFED_FILES => vec![warning(
            &format!("{}.files", *FIELD),
            &format!(
                "holds {} diffs and the docs allow at most {MAX_DIFFED_FILES}",
                files.len()
            ),
        )],
        _ => Vec::new(),
    }
}

fn paths_missing(diff: &JsValue) -> Vec<PluginIssue> {
    let files = items(diff, "files").map_or(0, Vec::len);
    let more_files = diff
        .get("moreFiles")
        .and_then(JsValue::as_f64)
        .unwrap_or(0.0);
    if items(diff, "changedFiles").is_none() && (files > 0 || more_files > 0.0) {
        return vec![warning(
            &format!("{}.changedFiles", *FIELD),
            "is missing, it is present whenever files holds a diff or moreFiles is above zero",
        )];
    }
    Vec::new()
}

fn relative_paths(diff: &JsValue) -> Vec<PluginIssue> {
    let changed = items(diff, "changedFiles").map_or_else(Vec::new, |paths| texts(paths));
    let mut found = not_absolute(&changed, &format!("{}.changedFiles", *FIELD));
    for (index, file) in items(diff, "files").into_iter().flatten().enumerate() {
        let path = file
            .get("filePath")
            .and_then(JsValue::as_str)
            .unwrap_or_default();
        found.extend(
            not_absolute(&[path], &format!("{}.files.{index}", *FIELD))
                .into_iter()
                .map(|issue| PluginIssue {
                    path: format!("{}.files.{index}.filePath", *FIELD),
                    ..issue
                }),
        );
    }
    found
}

fn skipped_with_diffs(diff: &JsValue) -> Vec<PluginIssue> {
    let skipped = diff.get("skipped").and_then(JsValue::as_bool) == Some(true);
    if skipped && items(diff, "files").is_some_and(|files| !files.is_empty()) {
        return vec![warning(
            &format!("{}.skipped", *FIELD),
            "is set, so Claude Code takes no diff and files should be empty",
        )];
    }
    Vec::new()
}

/// What the table states as a limit or an invariant; a violation is a warning that keeps the data.
const CHECKS: [Check; 5] = [
    too_many_paths,
    too_many_diffs,
    paths_missing,
    relative_paths,
    skipped_with_diffs,
];

/// Reads `tool_response.bashEditDiff` of a Bash PostToolUse call (hooks.md, "Bash"): "the changed
/// files". A call without the field has no diff (background and read-only commands carry none, and
/// recording may be off); a wrong type is an error issue and nothing throws; a broken limit is a
/// warning and the data is kept whole.
#[must_use]
pub fn parse_bash_edit_diff(tool_response: &JsValue) -> BashEditDiffParse {
    if !is_record(tool_response) {
        return BashEditDiffParse {
            diff: None,
            issues: vec![error(RESPONSE_ROOT, "expected an object")],
        };
    }
    let bash_edit_diff = tool_response
        .get("bashEditDiff")
        .unwrap_or(&JsValue::Undefined);
    if bash_edit_diff.is_undefined() {
        return BashEditDiffParse {
            diff: None,
            issues: Vec::new(),
        };
    }
    match SCHEMA.safe_parse(bash_edit_diff) {
        Err(failure) => BashEditDiffParse {
            diff: None,
            issues: issues_of(&FIELD, &failure),
        },
        Ok(data) => BashEditDiffParse {
            issues: CHECKS.iter().flat_map(|check| check(&data)).collect(),
            diff: Some(data),
        },
    }
}

/// How far a list can be relied on: "how complete and how reliable that list is".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashEditDiffTrust {
    /// `false` when the diff is incomplete or could not be taken (`unavailable`), or was not taken (`skipped`).
    pub complete: bool,
    /// `false` when another Bash call ran in the repository meanwhile, so some changes may be its (`shared`).
    pub attributable: bool,
}

/// The two flags that say a list is incomplete and the one that says it may hold another command's changes.
#[must_use]
pub fn bash_edit_diff_trust(diff: &JsValue) -> BashEditDiffTrust {
    let set = |name: &str| diff.get(name).and_then(JsValue::as_bool) == Some(true);
    BashEditDiffTrust {
        complete: !set("unavailable") && !set("skipped"),
        attributable: !set("shared"),
    }
}

crate::registry::exports! {
    "rules/tools/responses/bash-edit-diff" {
        "parseBashEditDiff" => ok |tool_response: JsValue| parse_bash_edit_diff(&tool_response);
        "bashEditDiffTrust" => ok |diff: JsValue| bash_edit_diff_trust(&diff);
    }
}
