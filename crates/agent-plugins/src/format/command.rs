//! Port of `src/format/command.ts`, switched.
//!
//! Commands, "the older format" (plugins/components.md, "Commands"; skills.md, "Where skills
//! live"): a flat Markdown file that "takes the same frontmatter as skills" except `name` and
//! `paths`, or an entry of the manifest's `commands` object map (plugins/manifest-reference.md,
//! "commands"). Never throws; a refused command is an error issue and no command.

use serde::{Deserialize, Serialize};

use super::code_points::code_point_length;
use super::frontmatter::parse_frontmatter;
use super::issues::{PluginIssue, error, warning};
use super::manifest_schema::COMMAND_ENTRY_ONE_OF;
use super::skill_fields::read_invocable;
use super::skill_name::{SkillOrigin, plugin_prefix, refused_characters, reserved_namespace};
use crate::js::{JsObject, JsValue, trim};

/// A command, or the issues that refuse it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandRead {
    /// The `Command`; absent when the command is refused: an error issue says why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<JsObject>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Where a command file is: its origin, and its path below its `commands` folder.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandLayout {
    /// Where the command came from, as the host wrote it.
    pub origin: JsValue,
    /// The file's path below its `commands` folder, `/`-separated, with its extension: `db/migrate.md`.
    pub relative_path: String,
}

/// What an entry of the `commands` map is read with.
#[derive(Clone, Debug, Deserialize)]
pub struct CommandEntryContext {
    /// Where the command came from, as the host wrote it.
    pub origin: JsValue,
}

/// An entry of the manifest's `commands` object map (`CommandEntry`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandEntry {
    /// The file the command is read from.
    #[serde(default)]
    pub source: Option<String>,
    /// The body, inline.
    #[serde(default)]
    pub content: Option<String>,
    /// Replaces the file's `description`.
    #[serde(default)]
    pub description: Option<String>,
    /// Replaces the file's `argument-hint`.
    #[serde(default)]
    pub argument_hint: Option<String>,
    /// Replaces the file's `model`.
    #[serde(default)]
    pub model: Option<String>,
    /// Replaces the file's `allowed-tools`.
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
}

/// What a command is read from: its frontmatter fields and its body.
struct Parsed {
    fields: JsObject,
    body: String,
    problem: Option<String>,
    line: Option<usize>,
}

/// Skills.md: a command file "accepts the same fields except `name` and `paths`".
const NOT_TAKEN: [&str; 2] = ["name", "paths"];

/// The name's segments: a path's folders and file, or an entry's key, joined by `:` in the name.
const SEGMENT: &str = ":";

/// The longest command name, in characters (code points), joined segments included.
const MAX_NAME: usize = 256;

/// Whether Claude Code loads a command of this name (skills.md, "Characters a skill name can't
/// hold", "Names reserved for synced skills"): "A custom command or skill doesn't load when Claude
/// Code refuses its directory or file name". The same refusal applies to a manifest key, the "command
/// name" the docs say is refused (judgment: the docs name no other rule for a key). `anthropic-skills`
/// is reserved for synced skills; "a plugin named `anthropic-skills`: it loads", so a plugin command
/// is not refused for it.
#[must_use]
pub fn command_name_issues(
    at: &str,
    segments: &[String],
    origin: &SkillOrigin,
) -> Vec<PluginIssue> {
    let name = segments.join(SEGMENT);
    if segments.iter().any(String::is_empty) {
        let message = format!("command name \"{name}\" has an empty part; the command is refused");
        return vec![error(at, &message)];
    }
    let found: Vec<String> = segments
        .iter()
        .flat_map(|segment| refused_characters(segment))
        .collect();
    if !found.is_empty() {
        let message = format!(
            "command name has a character a command name can't hold ({}); the command is refused, rename it",
            found.join(", ")
        );
        return vec![error(at, &message)];
    }
    // errors.md, "Skill or command name refused": a name "over 256 characters" is refused (spec §4.2).
    if code_point_length(&name) > MAX_NAME {
        let message = format!(
            "command name is over {MAX_NAME} characters; the command is refused, rename it"
        );
        return vec![error(at, &message)];
    }
    let reserved = if matches!(origin, SkillOrigin::Plugin { .. }) {
        None
    } else {
        reserved_namespace(&name)
    };
    reserved.map_or_else(Vec::new, |reserved| {
        let message = format!(
            "command name uses \"{reserved}\", a name reserved for skills synced from claude.ai; the command does not load, rename it"
        );
        vec![error(at, &message)]
    })
}

fn parse_problem(path: &str, parsed: &Parsed) -> Vec<PluginIssue> {
    let Some(problem) = &parsed.problem else {
        return Vec::new();
    };
    let place = parsed
        .line
        .map_or_else(String::new, |line| format!(" (line {line})"));
    let message =
        format!("frontmatter {problem}{place}; the command loads with its body and no fields set");
    vec![warning(path, &message)]
}

struct Where<'a> {
    /// What an issue about the fields is reported against: the file, or the entry.
    path: &'a str,
    /// What an issue about the name is reported against.
    name_path: &'a str,
    segments: Vec<String>,
    origin: &'a JsValue,
    file: Option<&'a str>,
}

/// Name, fields and body into a `Command`; a refused name gives no command.
fn assemble(place: &Where<'_>, parsed: Parsed, overlay: JsObject) -> Result<CommandRead, String> {
    let origin =
        SkillOrigin::deserialize(place.origin.clone()).map_err(|problem| problem.to_string())?;
    let refused = command_name_issues(place.name_path, &place.segments, &origin);
    if !refused.is_empty() {
        return Ok(CommandRead {
            command: None,
            issues: refused,
        });
    }
    let untaken: Vec<String> = NOT_TAKEN.iter().map(|key| (*key).to_owned()).collect();
    let read = read_invocable(&parsed.fields, &parsed.body, &untaken);
    let mut issues = parse_problem(place.path, &parsed);
    issues.extend(
        read.problems
            .iter()
            .map(|found| warning(place.path, &format!("{} {}", found.key, found.reason))),
    );
    let name = place.segments.join(SEGMENT);
    let mut command = read.frontmatter;
    command.extend(overlay);
    let invoked = format!("{}{name}", plugin_prefix(&origin));
    command.insert("kind".to_owned(), "command".into());
    command.insert("name".to_owned(), name.into());
    command.insert("command".to_owned(), invoked.into());
    command.insert("origin".to_owned(), place.origin.clone());
    if let Some(file) = place.file {
        command.insert("path".to_owned(), file.into());
    }
    command.insert("body".to_owned(), parsed.body.into());
    Ok(CommandRead {
        command: Some(command),
        issues,
    })
}

/// `file.replace(/\.md$/i, '')`.
fn without_extension(file: &str) -> &str {
    let cut = file.len().saturating_sub(3);
    match file.get(cut..) {
        Some(tail) if tail.eq_ignore_ascii_case(".md") => file.get(..cut).unwrap_or(file),
        _ => file,
    }
}

fn parsed_text(text: &str) -> Parsed {
    let read = parse_frontmatter(text);
    Parsed {
        fields: read.fields,
        body: read.body,
        problem: read.problem,
        line: read.line,
    }
}

/// Read a command file. The name is its path below `commands/` without the extension, the folders
/// joined by `:` (`db/migrate.md` is `db:migrate`), after the plugin prefix for a plugin's file.
///
/// # Errors
///
/// When the layout's origin is no `SkillOrigin`.
pub fn read_command(path: &str, text: &str, layout: &CommandLayout) -> Result<CommandRead, String> {
    let segments = without_extension(&layout.relative_path)
        .split('/')
        .map(str::to_owned)
        .collect();
    let place = Where {
        path,
        name_path: path,
        segments,
        origin: &layout.origin,
        file: Some(path),
    };
    assemble(&place, parsed_text(text), JsObject::new())
}

fn present(text: Option<&str>) -> Option<String> {
    text.map(trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_owned)
}

/// The fields an entry sets. Judgment (spec/judgments.md): a field the entry sets replaces the same
/// field of its `source` file's frontmatter; the docs do not say which wins.
fn overlay_of(entry: &CommandEntry) -> JsObject {
    let mut overlay = JsObject::new();
    let fields = [
        ("description", &entry.description),
        ("argumentHint", &entry.argument_hint),
        ("model", &entry.model),
    ];
    for (key, value) in fields {
        if let Some(value) = present(value.as_deref()) {
            overlay.insert(key.to_owned(), value.into());
        }
    }
    if let Some(rules) = &entry.allowed_tools {
        let kept = rules
            .iter()
            .filter_map(|rule| present(Some(rule)))
            .map(JsValue::from);
        overlay.insert("allowedTools".to_owned(), JsValue::Array(kept.collect()));
    }
    overlay
}

/// The entry's body and fields: the inline content as written, or the file's text parsed.
fn entry_parsed(entry: &CommandEntry, source_text: Option<&str>) -> Option<Parsed> {
    if let Some(content) = &entry.content {
        return Some(Parsed {
            fields: JsObject::new(),
            body: content.clone(),
            problem: None,
            line: None,
        });
    }
    source_text.map(parsed_text)
}

/// Build a `Command` from one entry of the manifest's `commands` map. `key` is the command name
/// after the plugin prefix. `source_text` is the text of the entry's `source` file, `None` when
/// the file is not there. Exactly one of `source` and `content` must be set (the manifest says an
/// entry with both or neither "fails validation"). Judgment (spec/judgments.md): `content` is the
/// body as written, not parsed for frontmatter ("Inline Markdown for the command body").
///
/// # Errors
///
/// When the context's origin is no `SkillOrigin`.
pub fn read_command_entry(
    key: &str,
    entry: &CommandEntry,
    source_text: Option<&str>,
    ctx: &CommandEntryContext,
) -> Result<CommandRead, String> {
    let at = format!("commands.{key}");
    if entry.source.is_none() == entry.content.is_none() {
        return Ok(CommandRead {
            command: None,
            issues: vec![error(&at, COMMAND_ENTRY_ONE_OF)],
        });
    }
    let Some(parsed) = entry_parsed(entry, source_text) else {
        let source = entry.source.as_deref().unwrap_or("undefined");
        let message = format!("source \"{source}\" was not found in the plugin folder");
        return Ok(CommandRead {
            command: None,
            issues: vec![error(&at, &message)],
        });
    };
    let place = Where {
        path: entry.source.as_deref().unwrap_or(&at),
        name_path: &at,
        segments: vec![key.to_owned()],
        origin: &ctx.origin,
        file: entry.source.as_deref(),
    };
    assemble(&place, parsed, overlay_of(entry))
}

crate::registry::exports! {
    "format/command" {
        "readCommand" => fallible |path: String, text: String, layout: CommandLayout|
            read_command(&path, &text, &layout);
        "readCommandEntry" => fallible |key: String, entry: CommandEntry, source_text: Option<String>, ctx: CommandEntryContext|
            read_command_entry(&key, &entry, source_text.as_deref(), &ctx);
    }
}
