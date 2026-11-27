//! Port of `src/format/output-style-select.ts`, switched.
//!
//! Which output style is active, and what that means for the system prompt (output-styles.md,
//! "Change your output style" and "How output styles work"). Pure: the host reads its settings and
//! passes the values in. A selection hands back the host's own style object (`StyleSelection.style`
//! is the very value given), so the boundary answers a custom style by its place in `styles`
//! (`Selected.at`) and the wrapper puts the host's object there.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::output_style::OutputStyle;
use super::skill_fields::first_content_line;
use crate::js::trim;

/// A built-in style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BuiltinStyle {
    /// Its id.
    pub id: &'static str,
    /// The docs' own words for it.
    pub description: &'static str,
}

/// `default` and the four built-in styles, in the order output-styles.md lists them. The
/// descriptions are the docs' own words ("What changes" column; Default section).
pub const BUILTIN_OUTPUT_STYLES: [BuiltinStyle; 5] = [
    BuiltinStyle {
        id: "default",
        description: "Claude Code's standard instructions for completing software engineering tasks",
    },
    BuiltinStyle {
        id: "Proactive",
        description: "Claude starts work right away and makes reasonable assumptions rather than asking about routine decisions",
    },
    BuiltinStyle {
        id: "Concise",
        description: "Responses lead with the result and leave out preamble, narration, and recaps",
    },
    BuiltinStyle {
        id: "Explanatory",
        description: "Claude adds short `Insight` blocks that explain the choices behind the code it writes",
    },
    BuiltinStyle {
        id: "Learning",
        description: "Claude explains its choices and leaves small pieces of code for you to write yourself",
    },
];

/// The id Default has in the `/output-style` list.
pub const DEFAULT_OUTPUT_STYLE_ID: &str = "default";

/// The style in force: Default, a built-in one, or a custom one (`style` is set).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSelection {
    /// The style's id.
    pub id: String,
    /// Whether Claude Code's software engineering instructions stay in the prompt.
    pub keep_coding_instructions: bool,
    /// The custom style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<OutputStyle>,
    /// Where the custom style is in the list it was selected from; the core's own, never crossed.
    #[serde(skip)]
    pub at: Option<usize>,
}

fn builtin_selection(builtin: &BuiltinStyle) -> StyleSelection {
    StyleSelection {
        id: builtin.id.to_owned(),
        keep_coding_instructions: true,
        style: None,
        at: None,
    }
}

fn custom_selection(at: usize, style: &OutputStyle) -> StyleSelection {
    StyleSelection {
        id: style.id.clone(),
        keep_coding_instructions: style.keep_coding_instructions,
        style: Some(style.clone()),
        at: Some(at),
    }
}

/// What decides the active style.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct OutputStyleRequest {
    /// The `outputStyle` setting, as the merged settings hold it.
    #[serde(default)]
    pub setting: Option<String>,
    /// The argument of `/output-style <style>` given in this session.
    #[serde(default)]
    pub command: Option<String>,
    /// The custom styles in precedence order: when two share an id the first wins.
    #[serde(default)]
    pub styles: Vec<OutputStyle>,
}

fn candidates(styles: &[OutputStyle]) -> impl Iterator<Item = StyleSelection> + '_ {
    BUILTIN_OUTPUT_STYLES.iter().map(builtin_selection).chain(
        styles
            .iter()
            .enumerate()
            .map(|(at, style)| custom_selection(at, style)),
    )
}

/// The style the argument of `/output-style <style>` names, a built-in or a custom one, without
/// regard to case; `None` when it names none, which leaves the choice to the setting.
#[must_use]
pub fn find_output_style(styles: &[OutputStyle], argument: &str) -> Option<StyleSelection> {
    let wanted = trim(argument).to_lowercase();
    candidates(styles).find(|entry| entry.id.to_lowercase() == wanted)
}

/// The active style. A `force-for-plugin` style wins over everything ("Overrides the user's
/// `outputStyle` setting"). Otherwise `/output-style` (which ignores case) wins over the setting,
/// which is case-sensitive: "A value that doesn't match a style name exactly, such as
/// `explanatory`, gives you the Default style."
#[must_use]
pub fn resolve_output_style(request: &OutputStyleRequest) -> StyleSelection {
    let styles = &request.styles;
    if let Some((at, forced)) = styles
        .iter()
        .enumerate()
        .find(|(_, style)| style.force_for_plugin)
    {
        return custom_selection(at, forced);
    }
    let by_command = request
        .command
        .as_deref()
        .and_then(|command| find_output_style(styles, command));
    by_command
        .or_else(|| {
            let setting = request.setting.as_deref()?;
            candidates(styles).find(|entry| entry.id == setting)
        })
        .unwrap_or_else(|| StyleSelection {
            id: DEFAULT_OUTPUT_STYLE_ID.to_owned(),
            keep_coding_instructions: true,
            style: None,
            at: None,
        })
}

/// Which system prompt Claude Code builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptKind {
    /// The full prompt.
    Full,
    /// The shorter prompt, which has no software engineering section.
    Short,
}

/// Whether the final prompt carries Claude Code's software engineering instructions. On the full
/// prompt a custom style drops them unless `keep-coding-instructions` is true; on the short prompt
/// there is nothing to keep or drop.
#[must_use]
pub fn include_coding_instructions(selection: &StyleSelection, prompt_kind: PromptKind) -> bool {
    let has_section = match prompt_kind {
        PromptKind::Full => true,
        PromptKind::Short => false,
    };
    has_section && selection.keep_coding_instructions
}

/// One row of `/output-style`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutputStyleChoice {
    /// The style's id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The text beside the name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The style in force, which `/output-style` with no argument marks.
    pub current: bool,
}

/// The text beside a custom style's name: its `description`, else the first non-empty line of its
/// body (spec J6.3, oracle 2.1.325); nothing for an empty body.
fn describe(style: &OutputStyle) -> Option<String> {
    style
        .description
        .clone()
        .or_else(|| first_content_line(&style.instructions))
}

/// What `/output-style` lists with no argument and the `/config` picker shows: Default, the
/// built-in styles, then each custom style with its `name` and `description`. It lists the same pool
/// `resolve_output_style` selects from, so a custom style that reuses a built-in id or an earlier
/// style's id is left out (the first wins) and exactly one row is current.
#[must_use]
pub fn list_output_styles(
    styles: &[OutputStyle],
    current: &StyleSelection,
) -> Vec<OutputStyleChoice> {
    let mut seen: HashSet<&str> = BUILTIN_OUTPUT_STYLES
        .iter()
        .map(|builtin| builtin.id)
        .collect();
    let builtin = BUILTIN_OUTPUT_STYLES
        .iter()
        .map(|builtin| OutputStyleChoice {
            id: builtin.id.to_owned(),
            name: builtin.id.to_owned(),
            description: Some(builtin.description.to_owned()),
            current: builtin.id == current.id,
        });
    let mut rows: Vec<OutputStyleChoice> = builtin.collect();
    for style in styles {
        if seen.insert(style.id.as_str()) {
            rows.push(OutputStyleChoice {
                id: style.id.clone(),
                name: style.name.clone(),
                description: describe(style),
                current: style.id == current.id,
            });
        }
    }
    rows
}

/// A selection as it crosses: the custom style by its place in the list it was selected from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selected {
    /// The style's id.
    pub id: String,
    /// Whether Claude Code's software engineering instructions stay in the prompt.
    pub keep_coding_instructions: bool,
    /// The custom style's place in `styles`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<usize>,
}

fn selected(selection: StyleSelection) -> Selected {
    Selected {
        id: selection.id,
        keep_coding_instructions: selection.keep_coding_instructions,
        at: selection.at,
    }
}

crate::registry::exports! {
    "format/output-style-select" {
        "BUILTIN_OUTPUT_STYLES" => ok || BUILTIN_OUTPUT_STYLES;
        "DEFAULT_OUTPUT_STYLE_ID" => ok || DEFAULT_OUTPUT_STYLE_ID;
        "findOutputStyle" => ok |styles: Vec<OutputStyle>, argument: String| find_output_style(&styles, &argument).map(selected);
        "resolveOutputStyle" => ok |request: OutputStyleRequest| selected(resolve_output_style(&request));
        "includeCodingInstructions" => ok |selection: StyleSelection, prompt_kind: PromptKind|
            include_coding_instructions(&selection, prompt_kind);
        "listOutputStyles" => ok |styles: Vec<OutputStyle>, current: StyleSelection| list_output_styles(&styles, &current);
    }
}
