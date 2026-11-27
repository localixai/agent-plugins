//! Ported from `src/loading/validate/json-files.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! The JSON files a plugin declares (oracle 2.1.319): `hooks/hooks.json` and the files `hooks`
//! names, `.mcp.json` and the files `mcpServers` names. A file that is missing is not reported
//! here: the manifest's path check does that for a declared one, and a default one is optional.

use indexmap::IndexSet;

use crate::format::read_json::{
    BOM_PARSE_ERROR, JsonRead, invalid_json_syntax, opens_with_bom, parse_json,
};
use crate::format::validate::finding::{
    Finding, ValidationIssue, error_at, error_in, in_file, warning_in,
};
use crate::format::validate::hooks_check::{GUARDED_TAIL, hooks_map_findings};
use crate::format::validate::mcp_check::mcp_findings;
use crate::format::validate::received::kind_of;
use crate::js::JsValue;
use crate::loading::validate::links::{
    HOOKS_LINK, MCP_FILE_CAP_BYTES, MCP_NOT_REGULAR, MCP_PAST_CAP, is_link,
};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::protocol::is_record::is_record;

const BREAKS_LOAD: &str = "At runtime this breaks the entire plugin load.";
const SKIPPED_MCP: &str =
    "The plugin loader logs this at debug level and loads no servers from the file.";

/// A file's value, or the issues that stop it being read.
enum Parsed {
    Value(JsValue),
    Issues(Vec<ValidationIssue>),
}

/// How a file's parse error is worded.
struct How<'a> {
    tail: &'a str,
    bom: &'a str,
}

/// The file read once, as bytes: the byte-order mark `readText` would drop is seen, and the rest is
/// the UTF-8 text `readText` gives (the port's contract), so the file is never read twice.
async fn parsed(
    files: &dyn PluginFiles,
    path: &str,
    how: How<'_>,
    signal: Option<&Signal>,
) -> Option<Parsed> {
    let How { tail, .. } = how;
    if !files.exists(path, signal).await {
        return None;
    }
    let bytes = files.read_bytes(path, signal).await;
    if let Some(bytes) = &bytes
        && opens_with_bom(bytes)
    {
        let message = invalid_json_syntax(&format!("{} {tail}", how.bom));
        return Some(Parsed::Issues(vec![error_in(path, "json", &message)]));
    }
    let read = match bytes {
        None => JsonRead::Problem {
            problem: "the file could not be read".to_owned(),
            missing: true,
        },
        Some(bytes) => parse_json(&String::from_utf8_lossy(&bytes)),
    };
    match read {
        JsonRead::Value { value } => Some(Parsed::Value(value)),
        JsonRead::Problem { problem, .. } => {
            let message = invalid_json_syntax(&format!("{problem}. {tail}"));
            Some(Parsed::Issues(vec![error_in(path, "json", &message)]))
        }
    }
}

const HOOKS_BOM: &str = BOM_PARSE_ERROR;
const MCP_BOM: &str = "the file starts with a UTF-8 byte-order mark (BOM), which this file (unlike plugin.json) must not have — re-save it as UTF-8 without BOM.";

const MISSING_HOOKS: &str =
    "hooks.json must have `hooks` (the hook matchers) or `modules` (hooks modules), or both";

const GUARD_KEYS: [&str; 2] = ["PreToolUse", "PermissionRequest"];

fn outside() -> String {
    format!(
        "PreToolUse/PermissionRequest is declared at the top level, outside the \"hooks\" object{GUARDED_TAIL}"
    )
}

/// Something is written there: anything but nothing, `null` or an empty list.
fn declared(value: Option<&JsValue>) -> bool {
    value.is_some_and(|value| {
        !value.is_nullish() && value.as_array().is_none_or(|items| !items.is_empty())
    })
}

fn hooks_file_findings(value: &JsValue) -> Vec<Finding> {
    let Some(record) = value.as_object().filter(|_| is_record(value)) else {
        let message = format!(
            "Invalid input: expected object, received {}",
            kind_of(value)
        );
        return vec![error_at("root", &message)];
    };
    let modules = record
        .get("modules")
        .and_then(JsValue::as_array)
        .is_some_and(|items| !items.is_empty());
    let hooks = record.get("hooks").filter(|hooks| !hooks.is_undefined());
    // oracle 2.1.325: a guard key beside a `hooks` object is refused as one without it.
    let stray = GUARD_KEYS.iter().any(|key| declared(record.get(*key)))
        || hooks.is_some_and(|hooks| hooks.is_array() && declared(Some(hooks)));
    if stray {
        return vec![error_at("hooks", &outside())];
    }
    let Some(hooks) = hooks else {
        return if modules {
            Vec::new()
        } else {
            vec![error_at("root", MISSING_HOOKS)]
        };
    };
    match hooks.as_object().filter(|_| is_record(hooks)) {
        None => {
            let message = format!(
                "Invalid input: expected record, received {}",
                kind_of(hooks)
            );
            vec![error_at("hooks", &message)]
        }
        Some(map) => hooks_map_findings(map, None),
    }
}

/// Findings of one hooks file.
pub async fn hooks_file_issues(
    files: &dyn PluginFiles,
    path: &str,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let folder = path.rfind('/').and_then(|at| path.get(..at));
    if let Some(folder) = folder
        && is_link(files, folder, signal).await
    {
        return vec![warning_in(path, "directory", HOOKS_LINK.folder)];
    }
    if is_link(files, path, signal).await {
        return vec![warning_in(path, "directory", HOOKS_LINK.file)];
    }
    let bom = format!("{HOOKS_BOM}.");
    let how = How {
        tail: BREAKS_LOAD,
        bom: &bom,
    };
    match parsed(files, path, how, signal).await {
        None => Vec::new(),
        Some(Parsed::Issues(issues)) => issues,
        Some(Parsed::Value(value)) => in_file(path, &hooks_file_findings(&value), ""),
    }
}

/// Findings of one MCP servers file.
pub async fn mcp_file_issues(
    files: &dyn PluginFiles,
    path: &str,
    declared: &IndexSet<String>,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let stat = files.stat(path, signal).await;
    if stat.as_ref().is_some_and(|stat| stat.is_symlink) {
        return vec![warning_in(path, "directory", MCP_NOT_REGULAR)];
    }
    // A regular file over the cap is not read (spec §8 validator table, J8.27).
    #[expect(clippy::cast_precision_loss, reason = "a file size is far below 2^53")]
    if stat.is_some_and(|stat| stat.size as f64 > MCP_FILE_CAP_BYTES) {
        return vec![warning_in(path, "directory", MCP_PAST_CAP)];
    }
    let how = How {
        tail: SKIPPED_MCP,
        bom: MCP_BOM,
    };
    match parsed(files, path, how, signal).await {
        None => Vec::new(),
        Some(Parsed::Issues(issues)) => issues,
        Some(Parsed::Value(value)) => in_file(path, &mcp_findings(&value, declared), ""),
    }
}
