//! Port of `src/format/output-style-command.ts`, switched.
//!
//! `/output-style` and `/config outputStyle=<style>` with the two refusals of spec §6.2 item 4
//! (errors.md, "Custom output styles can't be selected over Remote Control" and "Output styles are
//! saved to local settings which this session doesn't load"). Pure: the host says where the turn
//! came from and which setting sources the session loads, then shows the reply and, for
//! `selected`, writes `outputStyle` to the project's `.claude/settings.local.json`.

use serde::{Deserialize, Serialize};

use super::output_style::OutputStyle;
use super::output_style_select::{
    BUILTIN_OUTPUT_STYLES, OutputStyleChoice, StyleSelection, find_output_style, list_output_styles,
};

/// The notice a turn over Remote Control or a relayed message gets beside a list or an unknown name.
pub const CUSTOM_STYLES_NOT_SELECTABLE: &str = "Custom output styles can't be selected over Remote Control or from a relayed message. Select one in the session itself, or pick a built-in style here.";

/// The refusal of a selection in a session that does not load the `local` setting source.
pub const LOCAL_SETTINGS_NOT_LOADED: &str = "Output styles are saved to local settings (.claude/settings.local.json), which this session doesn't load, so the style can't be changed here.";

/// A `/output-style` command.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputStyleCommand {
    /// The argument of `/output-style <style>` or `/config outputStyle=<style>`; absent lists the
    /// styles.
    #[serde(default)]
    pub argument: Option<String>,
    /// The custom styles in precedence order, as `resolveOutputStyle` takes them.
    pub styles: Vec<OutputStyle>,
    /// The style in force, which the list marks.
    pub current: StyleSelection,
    /// The turn came over Remote Control or in a message relayed into the session: only built-in
    /// styles.
    #[serde(default)]
    pub relayed: Option<bool>,
    /// Whether the session's setting sources include `local` (`--setting-sources`, SDK
    /// `settingSources`); default true.
    #[serde(default)]
    pub loads_local_settings: Option<bool>,
}

/// What the command answers: a list, a selection to write, an unknown name, or a refusal.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct OutputStyleReply {
    /// With no argument: the styles that can be selected, the current one marked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<OutputStyleChoice>>,
    /// The style the argument names; the host writes its id as `outputStyle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<StyleSelection>,
    /// The argument named no selectable style: the reply of an unknown name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown: Option<String>,
    /// Nothing is selected and nothing written: the text the user reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Added to a list or an unknown-name reply over Remote Control or a relayed message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

fn is_builtin(id: &str) -> bool {
    BUILTIN_OUTPUT_STYLES.iter().any(|builtin| builtin.id == id)
}

/// The reply of `/output-style`, with or without an argument.
#[must_use]
pub fn output_style_command(command: &OutputStyleCommand) -> OutputStyleReply {
    let relayed = command.relayed.unwrap_or(false);
    let styles: &[OutputStyle] = if relayed { &[] } else { &command.styles };
    let notice = relayed.then(|| CUSTOM_STYLES_NOT_SELECTABLE.to_owned());
    let Some(argument) = &command.argument else {
        let choices = list_output_styles(styles, &command.current)
            .into_iter()
            .filter(|choice| !relayed || is_builtin(&choice.id))
            .collect();
        return OutputStyleReply {
            choices: Some(choices),
            notice,
            ..OutputStyleReply::default()
        };
    };
    if command.loads_local_settings == Some(false) {
        return OutputStyleReply {
            refusal: Some(LOCAL_SETTINGS_NOT_LOADED.to_owned()),
            ..OutputStyleReply::default()
        };
    }
    match find_output_style(styles, argument) {
        None => OutputStyleReply {
            unknown: Some(argument.clone()),
            notice,
            ..OutputStyleReply::default()
        },
        Some(selected) => OutputStyleReply {
            selected: Some(selected),
            ..OutputStyleReply::default()
        },
    }
}

crate::registry::exports! {
    "format/output-style-command" {
        "CUSTOM_STYLES_NOT_SELECTABLE" => ok || CUSTOM_STYLES_NOT_SELECTABLE;
        "LOCAL_SETTINGS_NOT_LOADED" => ok || LOCAL_SETTINGS_NOT_LOADED;
        "outputStyleCommand" => ok |command: OutputStyleCommand| output_style_command(&command);
    }
}
