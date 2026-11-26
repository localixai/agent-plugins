//! Port of `src/format/output-style.ts`, switched.
//!
//! A custom output style (output-styles.md, "Create a custom output style"): a Markdown file whose
//! frontmatter is metadata and whose body is the instructions Claude Code sends with every request.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::frontmatter::parse_frontmatter;
use super::frontmatter_values::{FieldRead, read_flag, read_text};
use super::issues::{PluginIssue, warning};
use crate::js::{JsValue, array_join, compare_code_units, trim};

/// A custom output style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputStyle {
    /// `name`, else the file name without `.md`.
    pub name: String,
    /// What `/output-style` and the `outputStyle` setting call it: `<plugin>:<name>` for a plugin
    /// style (plugins/components.md, "Output styles"), `name` for a user, project or managed
    /// style.
    pub id: String,
    /// What it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `keep-coding-instructions`, default `false`.
    pub keep_coding_instructions: bool,
    /// `force-for-plugin`, default `false`, and always `false` for a style that is not a plugin's.
    pub force_for_plugin: bool,
    /// The Markdown body: the instructions for Claude.
    pub instructions: String,
    /// The path as the host gave it.
    pub path: String,
}

/// Where the file was found. Only a plugin style is prefixed and may be forced.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct OutputStyleContext {
    /// The plugin the file ships in; absent for a user, project or managed style.
    #[serde(default)]
    pub plugin: Option<String>,
}

/// What reading one style gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OutputStyleRead {
    /// The style.
    pub style: OutputStyle,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// `output-styles/terse.md` is `terse`; both `/` and `\` separate a path.
fn file_name(path: &str) -> String {
    let base = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem_end = base.len().saturating_sub(3);
    match base.get(stem_end..) {
        Some(end) if end.eq_ignore_ascii_case(".md") => base.get(..stem_end).unwrap_or(base),
        _ => base,
    }
    .to_owned()
}

const NAME_KEY: &str = "name";
const DESCRIPTION_KEY: &str = "description";
const KEEP_KEY: &str = "keep-coding-instructions";
const FORCE_KEY: &str = "force-for-plugin";

/// A `name` that is not a string is read as its JavaScript string conversion (spec J6.2, oracle
/// 2.1.319 and 2.1.322): `[a, b]` is `a,b`, a map `[object Object]`; a blank one is absent.
fn style_name(path: &str, value: &JsValue, issues: &mut Vec<PluginIssue>) -> Option<String> {
    let (JsValue::Array(_) | JsValue::Object(_)) = value else {
        return take(path, issues, read_text(NAME_KEY, value));
    };
    // JavaScript's conversion, written out: a list joins its items with `,`, a map is
    // `[object Object]`.
    let text = match value {
        JsValue::Array(items) => array_join(items, ","),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Object(_) => "[object Object]".to_owned(),
    };
    let text = trim(&text);
    (!text.is_empty()).then(|| text.to_owned())
}

/// The value of a read, its problem joining `issues` as a warning.
fn take<T>(path: &str, issues: &mut Vec<PluginIssue>, read: FieldRead<T>) -> Option<T> {
    if let Some(problem) = &read.problem {
        issues.push(warning(
            path,
            &format!("{} {}", problem.key, problem.reason),
        ));
    }
    read.value
}

/// Read one output style file. Never throws and never refuses a file (output-styles.md: "If the
/// YAML doesn't parse, the style still loads under its file name with no fields set"; "A misspelled
/// field is ignored without an error"). What the docs leave open is in spec/judgments.md: a YAML
/// error and a value of the wrong shape are warnings, never errors.
#[must_use]
pub fn read_output_style(path: &str, text: &str, context: &OutputStyleContext) -> OutputStyleRead {
    let parsed = parse_frontmatter(text);
    let mut issues: Vec<PluginIssue> = Vec::new();
    if let Some(problem) = &parsed.problem {
        let place = parsed
            .line
            .map_or_else(String::new, |line| format!(" (line {line})"));
        issues.push(warning(
            path,
            &format!(
                "frontmatter {problem}{place}; the style loads under its file name with no fields set"
            ),
        ));
    }
    let field = |key: &str| parsed.fields.get(key).unwrap_or(&JsValue::Undefined);
    let name = style_name(path, field(NAME_KEY), &mut issues).unwrap_or_else(|| file_name(path));
    let description = take(
        path,
        &mut issues,
        read_text(DESCRIPTION_KEY, field(DESCRIPTION_KEY)),
    );
    let keep = take(path, &mut issues, read_flag(KEEP_KEY, field(KEEP_KEY))).unwrap_or(false);
    let force = take(path, &mut issues, read_flag(FORCE_KEY, field(FORCE_KEY))).unwrap_or(false);
    let plugin = context.plugin.as_deref();
    OutputStyleRead {
        style: OutputStyle {
            id: plugin.map_or_else(|| name.clone(), |plugin| format!("{plugin}:{name}")),
            name,
            description,
            keep_coding_instructions: keep,
            force_for_plugin: plugin.is_some() && force,
            instructions: trim(&parsed.body).to_owned(),
            path: path.to_owned(),
        },
        issues,
    }
}

/// Of one plugin's styles that give the same `name`, exactly one is kept: the one whose path
/// relative to the plugin root sorts last by UTF-16 code unit (spec J6.7, oracle 2.1.325: of
/// `terse.md` and `terse2.md` both named `Terse`, `terse2.md` is listed). A fixed order keeps the
/// result independent of the file system's. The kept style stays where it was.
#[must_use]
pub fn last_per_name(styles: &[OutputStyle]) -> Vec<OutputStyle> {
    let mut kept: HashMap<&str, usize> = HashMap::new();
    for (index, style) in styles.iter().enumerate() {
        let held = kept
            .get(style.name.as_str())
            .and_then(|held| styles.get(*held));
        if held.is_none_or(|held| compare_code_units(&held.path, &style.path).is_lt()) {
            kept.insert(style.name.as_str(), index);
        }
    }
    styles
        .iter()
        .enumerate()
        .filter(|(index, style)| kept.get(style.name.as_str()) == Some(index))
        .map(|(_, style)| style.clone())
        .collect()
}

crate::registry::exports! {
    "format/output-style" {
        "readOutputStyle" => ok |path: String, text: String, context: Option<OutputStyleContext>| read_output_style(&path, &text, &context.unwrap_or_default());
        "lastPerName" => ok |styles: Vec<OutputStyle>| last_per_name(&styles);
    }
}
