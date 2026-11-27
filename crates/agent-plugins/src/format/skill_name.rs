//! Port of `src/format/skill-name.ts`, switched.
//!
//! How a skill gets its command name (skills.md, "How a skill gets its command name" and
//! "Characters a skill name can't hold"; plugins/components.md, "Skills").
//!
//! A `SkillLayout` is the shape of `src/format/skill-types.ts`, whose module is not in the core yet;
//! what this module reads of it is its origin (`kind`, and the `plugin` of a plugin's), its folder's
//! name and `fixedName`.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::code_points::code_point_length;
use super::issues::{PluginIssue, error, warning};
use crate::js::Pattern;

/// What the user types after `/`, with the issues found on the way (`SkillName`).
///
/// `path` of an issue is the field it is about: `name` (the frontmatter) or `folder`. A refused name's
/// message is its debug-log line, `[names] <name> is not offered` (the skill does not load) or
/// `[names] <value> is not shown for <skill>` (it loads without that `name`), hidden characters
/// spelled `\u{200B}`; a host writes it to the debug log and counts the lines for the startup
/// notice (`skillNamesRefused`). Any other message is a whole sentence that starts with the field
/// (`folder name uses ...`). An error means the skill does not load; a warning means it loads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkillName {
    /// The name the skill goes by: the frontmatter `name`, else the folder's name.
    pub name: String,
    /// What the user types after `/`.
    pub command: String,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Where a skill came from (`SkillOrigin`): only what the name rules read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SkillOrigin {
    /// A plugin's skill, namespaced by its plugin.
    Plugin {
        /// The plugin's name.
        plugin: String,
    },
    /// A managed skill.
    Managed,
    /// A skill of the user's.
    User,
    /// A skill of the project's.
    Project,
    /// A skill synced from claude.ai.
    Synced,
    /// The host's own skill.
    Bundled,
}

/// Where a `SKILL.md` sits: its origin and the name of its folder.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillLayout {
    /// Where the skill came from.
    pub origin: SkillOrigin,
    /// The folder's name.
    pub dir_name: String,
    /// `dir_name` is already the skill's name, as the plugin loader chose it, and a frontmatter `name`
    /// renames nothing. Claude Code 2.1.319 names a plugin skill that a scan finds (`skills/<folder>/`,
    /// or a folder of skills a manifest path lists) after its folder, and uses the frontmatter `name`
    /// only for a folder a path names itself and for the plugin-root `SKILL.md`; the loader makes that
    /// choice (spec/judgments.md, "oracle 2.1.319 overrides docs").
    #[serde(default)]
    pub fixed_name: Option<bool>,
}

/// Characters "such as" those the docs list: a zero-width space or a control character (Cc, and
/// the zero-width and bidi controls, which are all format characters, Cf), a private-use or
/// unassigned character (Co, Cn), a blank (the Z categories) and `<`/`>`. (`Cs`, lone surrogates,
/// is left out: a Rust string holds none.) A plain space is judged by its position, not here.
static REFUSED: Pattern = LazyLock::new(|| Regex::new(r"^[\p{Cc}\p{Cf}\p{Co}\p{Cn}\p{Z}<>]$").ok());

/// Which code points of `text` Claude Code refuses. A space is fine only alone between words.
fn refused_flags(text: &str) -> (Vec<char>, Vec<bool>) {
    let points: Vec<char> = text.chars().collect();
    let last_index = points.len().saturating_sub(1);
    let flags = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            if *point == ' ' {
                index == 0
                    || index == last_index
                    || index.checked_sub(1).and_then(|before| points.get(before)) == Some(&' ')
                    || points.get(index + 1) == Some(&' ')
            } else {
                is_refused_point(*point)
            }
        })
        .collect();
    (points, flags)
}

fn is_refused_point(point: char) -> bool {
    let mut buffer = [0_u8; 4];
    REFUSED
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(point.encode_utf8(&mut buffer)))
}

fn hex(point: char) -> String {
    format!("U+{:04X}", u32::from(point))
}

/// The refused characters of `text` as code points (never the raw text: it may be invisible).
#[must_use]
pub fn refused_characters(text: &str) -> Vec<String> {
    let (points, flags) = refused_flags(text);
    points
        .into_iter()
        .zip(flags)
        .filter(|(_, flag)| *flag)
        .map(|(point, _)| hex(point))
        .collect()
}

/// Each refused character becomes one hyphen.
fn hyphenate(text: &str) -> String {
    let (points, flags) = refused_flags(text);
    points
        .into_iter()
        .zip(flags)
        .map(|(point, flag)| if flag { '-' } else { point })
        .collect()
}

fn cannot_hold(field: &str, found: &[String]) -> String {
    format!(
        "{field} has a character a command name can't hold ({})",
        found.join(", ")
    )
}

/// skills.md, "Characters a skill name can't hold"; errors.md: "or be over 256 characters".
const MAX_NAME: usize = 256;

/// Whether Claude Code refuses `text` as a name: a character it can't hold, or over 256 code points.
fn is_refused(text: &str) -> bool {
    code_point_length(text) > MAX_NAME || refused_flags(text).1.contains(&true)
}

/// `text` as the debug log spells it (errors.md, "Skill or command name refused": "spells out some
/// hidden characters as codes, such as `\u{200B}`"): every refused character but the visible `<`
/// and `>` as its code point, so a blank at either end or a doubled space shows too.
fn spelled(text: &str) -> String {
    let (points, flags) = refused_flags(text);
    points
        .into_iter()
        .zip(flags)
        .map(|(point, flag)| {
            if flag && point != '<' && point != '>' {
                format!("\\u{{{:X}}}", u32::from(point))
            } else {
                point.to_string()
            }
        })
        .collect()
}

/// errors.md, "Skill or command name refused": the `[names]` line of a name a skill goes by.
fn not_offered(name: &str) -> String {
    format!("[names] {} is not offered", spelled(name))
}

/// The same, for a frontmatter `name` the skill loads without.
fn not_shown_for(value: &str, skill: &str) -> String {
    format!("[names] {} is not shown for {skill}", spelled(value))
}

struct Chosen {
    name: String,
    issues: Vec<PluginIssue>,
}

const NAMESPACE: &str = "anthropic-skills";

/// skills.md, "Names reserved for synced skills": `anthropic-skills` and `anthropic-skills:<name>`.
#[must_use]
pub fn reserved_namespace(text: &str) -> Option<&'static str> {
    (text == NAMESPACE || text.starts_with(&format!("{NAMESPACE}:"))).then_some(NAMESPACE)
}

/// skills.md, "Choose where skills load": a folder named `synced`, "in any capitalization".
fn reserved_folder(text: &str) -> Option<&'static str> {
    (text.to_lowercase() == "synced").then_some("synced")
}

/// A name that may hold a reserved word.
struct Reserved<'a> {
    /// The issue's `path`, and the first word of its message.
    path: &'static str,
    label: &'static str,
    text: Option<&'a str>,
    fix: &'static str,
    /// The reserved word of `text`, when it holds one.
    reserved: &'a dyn Fn(&str) -> Option<&'static str>,
}

/// The error of a reserved name: it refuses the skill (skills.md, "Names reserved for synced skills").
fn reserved_errors(field: &Reserved<'_>) -> Vec<PluginIssue> {
    let word = field.text.and_then(field.reserved);
    let Some(word) = word else {
        return Vec::new();
    };
    vec![error(
        field.path,
        &format!(
            "{} uses \"{word}\", a name reserved for skills synced from claude.ai; the skill does not load, {}",
            field.label, field.fix
        ),
    )]
}

/// Outside a plugin (skills.md, "Characters a skill name can't hold"; errors.md, "Skill or command
/// name refused"): a folder name Claude Code refuses keeps the skill from loading, an error; a
/// refused frontmatter `name` is ignored, a warning, and the skill loads under its folder's name. A
/// reserved folder or `name` refuses the skill. The folder `synced` is reserved only where Claude
/// Code authors no `synced` folder of its own: the user and project locations, not a synced skill.
fn strict(reserves_synced_folder: bool, dir_name: &str, declared: Option<&str>) -> Chosen {
    let ignored = declared.is_some_and(is_refused);
    let mut issues = Vec::new();
    if is_refused(dir_name) {
        issues.push(error("folder", &not_offered(dir_name)));
    }
    issues.extend(reserved_errors(&Reserved {
        path: "folder",
        label: "folder name",
        text: Some(dir_name),
        fix: "rename the folder",
        reserved: &|text| {
            reserved_namespace(text).or_else(|| {
                if reserves_synced_folder {
                    reserved_folder(text)
                } else {
                    None
                }
            })
        },
    }));
    if let Some(value) = declared.filter(|_| ignored) {
        issues.push(warning("name", &not_shown_for(value, dir_name)));
    }
    issues.extend(reserved_errors(&Reserved {
        path: "name",
        label: "name",
        text: declared,
        fix: "edit the name: line",
        reserved: &reserved_namespace,
    }));
    let name = if ignored {
        dir_name
    } else {
        declared.unwrap_or(dir_name)
    };
    Chosen {
        name: name.to_owned(),
        issues,
    }
}

fn strict_reserving_synced_folder(dir_name: &str, declared: Option<&str>) -> Chosen {
    strict(true, dir_name, declared)
}

fn strict_leaving_synced_folder(dir_name: &str, declared: Option<&str>) -> Chosen {
    strict(false, dir_name, declared)
}

/// In a plugin the skill always loads. A folder's refused characters become hyphens in the command
/// name; a refused frontmatter `name` is not used, with its `[names]` line (spec/judgments.md: the
/// folder names the skill). A long folder name is kept: the docs give a plugin folder only the
/// hyphen rule.
fn lenient(dir_name: &str, declared: Option<&str>) -> Chosen {
    if let Some(declared) = declared.filter(|text| !is_refused(text)) {
        return Chosen {
            name: declared.to_owned(),
            issues: Vec::new(),
        };
    }
    let mut issues = Vec::new();
    let name = hyphenate(dir_name);
    if let Some(declared) = declared {
        issues.push(warning("name", &not_shown_for(declared, &name)));
    }
    if name != dir_name {
        issues.push(warning(
            "folder",
            &format!(
                "{}; each becomes a hyphen in the command name ({name})",
                cannot_hold("folder name", &refused_characters(dir_name))
            ),
        ));
    }
    Chosen { name, issues }
}

/// The command name of a skill named by itself (the plugin root, or a manifest path to its folder),
/// oracle 2.1.319 (spec/judgments.md): the plugin's own `<plugin>:` prefix is dropped, then each
/// UTF-16 unit outside `[A-Za-z0-9_-]` is a hyphen, so `Plan Review` is `Plan-Review`, `other:x` is
/// `other-x` and an emoji is two. `claude plugin details` prints the name as written.
#[must_use]
pub fn self_named_command(name: &str, plugin: &str) -> String {
    let own = name.strip_prefix(&format!("{plugin}:")).unwrap_or(name);
    // No `u` flag on purpose: Claude Code replaces UTF-16 units, not code points.
    own.chars()
        .map(|point| {
            if point.is_ascii_alphanumeric() || point == '_' || point == '-' {
                point.to_string()
            } else {
                "-".repeat(point.len_utf16())
            }
        })
        .collect()
}

/// What differs per origin: what the command starts with, and how the name is chosen.
struct NameRule {
    /// What the command starts with.
    prefix: fn(&SkillOrigin) -> String,
    choose: fn(&str, Option<&str>) -> Chosen,
}

/// `<plugin>:` for a plugin's skill, else nothing.
#[must_use]
pub fn plugin_prefix(origin: &SkillOrigin) -> String {
    match origin {
        SkillOrigin::Plugin { plugin } => format!("{plugin}:"),
        SkillOrigin::Managed
        | SkillOrigin::User
        | SkillOrigin::Project
        | SkillOrigin::Synced
        | SkillOrigin::Bundled => String::new(),
    }
}

fn no_prefix(_origin: &SkillOrigin) -> String {
    String::new()
}

fn synced_prefix(_origin: &SkillOrigin) -> String {
    format!("{NAMESPACE}:")
}

/// One rule per origin (K1). Judgment (spec/judgments.md): a host's own skill takes the rules of a
/// skill outside a plugin, `anthropic-skills` included ("reserved in every session"); no folder of
/// its is scanned.
const fn rule_of(origin: &SkillOrigin) -> NameRule {
    match origin {
        SkillOrigin::Plugin { .. } => NameRule {
            prefix: plugin_prefix,
            choose: lenient,
        },
        SkillOrigin::Managed | SkillOrigin::User | SkillOrigin::Project => NameRule {
            prefix: no_prefix,
            choose: strict_reserving_synced_folder,
        },
        SkillOrigin::Synced => NameRule {
            prefix: synced_prefix,
            choose: strict_leaving_synced_folder,
        },
        SkillOrigin::Bundled => NameRule {
            prefix: no_prefix,
            choose: strict_leaving_synced_folder,
        },
    }
}

/// Resolve the name and the command of a skill from where it sits and its frontmatter `name`
/// (`declared`, as read; absent when `None`). A `name` that already starts with the prefix is
/// not prefixed again (v2.1.246 and later). Never fails: a refusal is an error issue, and `name`
/// and `command` are then the text as written, for the host to report.
#[must_use]
pub fn resolve_skill_name(layout: &SkillLayout, declared: Option<&str>) -> SkillName {
    let rule = rule_of(&layout.origin);
    let declared = if layout.fixed_name == Some(true) {
        None
    } else {
        declared
    };
    let Chosen { name, issues } = (rule.choose)(&layout.dir_name, declared);
    let prefix = (rule.prefix)(&layout.origin);
    let command = if name.starts_with(&prefix) {
        name.clone()
    } else {
        format!("{prefix}{name}")
    };
    SkillName {
        name,
        command,
        issues,
    }
}

crate::registry::exports! {
    "format/skill-name" {
        "selfNamedCommand" => ok |name: String, plugin: String| self_named_command(&name, &plugin);
        "resolveSkillName" => ok |layout: SkillLayout, declared: Option<String>| resolve_skill_name(&layout, declared.as_deref());
    }
}
