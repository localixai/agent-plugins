//! Port of `src/rules/path/tools.ts`, switched.
//!
//! The path tools and the field each carries its path in: `file_path` for Read, Edit and Write, `path`
//! for Grep and Glob, `notebook_path` for NotebookEdit (permissions.md, "Match by input parameter";
//! hooks.md, the PreToolUse input tables). A search tool's path is a directory and, when omitted, "the
//! current working directory will be used" (the SDK's own field description).

use serde::Serialize;

use super::posix::{HostPaths, absolute_path, normalize_absolute, to_posix_path};
use crate::js::JsValue;
use crate::registry::{Map, Set};
use crate::rules::types::{PathContext, PermissionBehavior, ToolCall};

/// A tool's path: the field it is in and how the tool uses it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PathTool {
    /// The field the path is in.
    pub field: &'static str,
    /// Whether the path is a directory searched.
    pub searches: bool,
    /// "file-reading tools, such as Read, Grep, and Glob" (permissions.md, "Network paths").
    pub reads: bool,
    /// The field of a pattern that, written as an absolute or `~` path, says where the search is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<&'static str>,
}

const READ_FIELD: &str = "file_path";
const EDIT_FIELD: &str = "file_path";
const WRITE_FIELD: &str = "file_path";
const GLOB_FIELD: &str = "path";
const GLOB_PATTERN_FIELD: &str = "pattern";
const GREP_FIELD: &str = "path";
const NOTEBOOK_FIELD: &str = "notebook_path";
/// The legacy MultiEdit tool is not in the SDK's types, so its field is a judgment: it edits one file
/// like Edit and takes the same `file_path` (spec/judgments.md).
const MULTI_EDIT_FIELD: &str = "file_path";
/// The SDK has no type for the LSP input; `filePath` ("The absolute or relative path to the file") is
/// the one path its declarations give, and a Read rule is matched against it (spec §7.6.6, J7.210).
const LSP_FIELD: &str = "filePath";

const fn tool(field: &'static str, searches: bool, reads: bool) -> PathTool {
    PathTool {
        field,
        searches,
        reads,
        pattern: None,
    }
}

/// The path tools, by name.
const PATH_TOOLS: [(&str, PathTool); 8] = [
    ("Read", tool(READ_FIELD, false, true)),
    ("Edit", tool(EDIT_FIELD, false, false)),
    ("MultiEdit", tool(MULTI_EDIT_FIELD, false, false)),
    ("Write", tool(WRITE_FIELD, false, false)),
    ("NotebookEdit", tool(NOTEBOOK_FIELD, false, false)),
    (
        "Glob",
        PathTool {
            pattern: Some(GLOB_PATTERN_FIELD),
            ..tool(GLOB_FIELD, true, true)
        },
    ),
    ("Grep", tool(GREP_FIELD, true, true)),
    ("LSP", tool(LSP_FIELD, false, false)),
];

/// The path tool of this name, if it is one.
#[must_use]
pub fn path_tool(tool_name: &str) -> Option<&'static PathTool> {
    PATH_TOOLS
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, tool)| tool)
}

/// `PATH_TOOLS` as the TypeScript's `Map`.
#[must_use]
pub fn path_tools() -> Map<String, PathTool> {
    Map {
        entries: PATH_TOOLS
            .iter()
            .map(|(name, tool)| ((*name).to_owned(), tool.clone()))
            .collect(),
    }
}

/// `ROOTED`, `/^(?:[\\/]|~(?=[\\/]|$)|[A-Za-z]:[\\/])/`: an absolute path, a drive path or a `~` path.
fn is_rooted(pattern: &str) -> bool {
    let mut characters = pattern.chars();
    match characters.next() {
        Some('\\' | '/') => true,
        Some('~') => matches!(characters.next(), None | Some('\\' | '/')),
        Some(drive) if drive.is_ascii_alphabetic() => {
            characters.next() == Some(':') && matches!(characters.next(), Some('\\' | '/'))
        }
        Some(_) | None => false,
    }
}

/// Where a search pattern written as an absolute or `~` path searches: its directories before the
/// first wildcard (`/etc/**\/*.conf` searches `/etc`), whatever the `path` field says, because the
/// pattern names the files (a judgment, spec/judgments.md). `undefined` for a relative pattern.
fn pattern_directory(pattern: Option<&JsValue>) -> Option<String> {
    let pattern = pattern?.as_str().filter(|pattern| is_rooted(pattern))?;
    let Some(wildcard) = pattern.find(['*', '?', '[', '{']) else {
        return Some(pattern.to_owned());
    };
    let head = pattern.get(..wildcard).unwrap_or_default();
    let cut = head.rfind(['/', '\\']).map_or(0, |at| at + 1);
    Some(head.get(..cut).unwrap_or_default().to_owned())
}

/// The text of the path a call acts on, as sent (a rooted Glob pattern's directory before the `path`
/// field); `None` for a tool without one or a non-string.
#[must_use]
pub fn path_text(call: &ToolCall) -> Option<String> {
    let tool = path_tool(&call.tool_name)?;
    if !matches!(call.input, JsValue::Object(_) | JsValue::Array(_)) {
        return None;
    }
    let rooted = tool
        .pattern
        .and_then(|pattern| pattern_directory(call.input.get(pattern)));
    if rooted.is_some() {
        return rooted;
    }
    match call.input.get(tool.field) {
        Some(JsValue::String(value)) => Some(value.clone()),
        Some(JsValue::Undefined) | None if tool.searches => Some(String::new()),
        Some(_) | None => None,
    }
}

/// The absolute POSIX path `call` acts on (a search without a path acts on the current directory).
#[must_use]
pub fn requested_path(call: &ToolCall, paths: &PathContext) -> Option<String> {
    let text = path_text(call)?;
    Some(if text.is_empty() {
        normalize_absolute(&to_posix_path(&paths.cwd, paths.platform))
    } else {
        absolute_path(
            &text,
            &HostPaths {
                cwd: paths.cwd.clone(),
                home: paths.home.clone(),
                platform: paths.platform,
            },
        )
    })
}

/// Whether the tool's path is a directory searched, so a pattern is tried against what is below it.
#[must_use]
pub fn searches_directory(tool_name: &str) -> bool {
    path_tool(tool_name).is_some_and(|tool| tool.searches)
}

/// Which call tools a path rule of `ruleTool` and `behavior` covers (the tool itself is always one):
/// "`Edit` rules apply to all built-in tools that edit files" (Write, NotebookEdit, the legacy
/// MultiEdit), Read rules reach Grep and Glob, a Read deny or ask rule also LSP ("Read, Grep, Glob,
/// LSP", tools-reference.md; matched against its `filePath`, J7.210), "An `Edit(...)` allow rule also grants read access to the same path", and "A
/// `Read(...)` deny rule also blocks the Edit and Write tools" but not NotebookEdit (permissions.md,
/// "What `Read` and `Edit` rules cover"; tools-reference.md).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Coverage {
    /// What an allow rule covers.
    pub allow: Vec<&'static str>,
    /// What an ask rule covers.
    pub ask: Vec<&'static str>,
    /// What a deny rule covers.
    pub deny: Vec<&'static str>,
}

impl Coverage {
    /// The tools a rule of `behavior` covers.
    #[must_use]
    pub fn of(&self, behavior: PermissionBehavior) -> &[&'static str] {
        match behavior {
            PermissionBehavior::Allow => &self.allow,
            PermissionBehavior::Ask => &self.ask,
            PermissionBehavior::Deny => &self.deny,
        }
    }
}

const EDITS: [&str; 4] = ["Edit", "Write", "NotebookEdit", "MultiEdit"];
const READS: [&str; 3] = ["Read", "Glob", "Grep"];
const READS_AND_LSP: [&str; 4] = ["Read", "Glob", "Grep", "LSP"];

/// The call tools whose path field may be absent: a rule that reaches such a call covers it (J7.210).
#[must_use]
pub fn pathless_calls() -> Set<&'static str> {
    Set {
        members: vec!["LSP"],
    }
}

/// `COVERAGE`, by rule tool.
#[must_use]
pub fn coverage() -> Map<&'static str, Coverage> {
    let read_deny: Vec<&str> = READS_AND_LSP
        .iter()
        .copied()
        .chain(["Edit", "Write", "MultiEdit"])
        .collect();
    Map {
        entries: vec![
            (
                "Read",
                Coverage {
                    allow: READS.to_vec(),
                    ask: READS_AND_LSP.to_vec(),
                    deny: read_deny,
                },
            ),
            (
                "Edit",
                Coverage {
                    allow: EDITS.iter().copied().chain(READS).collect(),
                    ask: EDITS.to_vec(),
                    deny: EDITS.to_vec(),
                },
            ),
        ],
    }
}

crate::registry::exports! {
    "rules/path/tools" {
        "pathText" => ok |call: ToolCall| path_text(&call);
        "requestedPath" => ok |call: ToolCall, paths: PathContext| requested_path(&call, &paths);
        "searchesDirectory" => ok |tool_name: String| searches_directory(&tool_name);
        "PATHLESS_CALLS" => ok pathless_calls;
        "COVERAGE" => ok coverage;
    }
}
