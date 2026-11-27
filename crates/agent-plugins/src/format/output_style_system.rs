//! Port of `src/format/output-style-system.ts`, switched.
//!
//! output-styles.md § How output styles work. The host supplies its private prompt parts; a style
//! replaces only coding-specific instructions unless it opts to keep them, and is appended intact.

use serde::Deserialize;

use super::output_style_select::{PromptKind, StyleSelection, include_coding_instructions};

/// The host's private prompt parts.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDefaults {
    /// The general instructions.
    pub general: Vec<String>,
    /// The coding-specific instructions.
    pub coding: Vec<String>,
    /// The prompt's kind; `full` when absent.
    #[serde(default)]
    pub prompt_kind: Option<PromptKind>,
}

/// output-styles.md § How output styles work. The host supplies its private prompt parts; a style
/// replaces only coding-specific instructions unless it opts to keep them, and is appended intact.
#[must_use]
pub fn output_style_system(selection: &StyleSelection, defaults: &PromptDefaults) -> Vec<String> {
    let mut parts = defaults.general.clone();
    if include_coding_instructions(selection, defaults.prompt_kind.unwrap_or(PromptKind::Full)) {
        parts.extend(defaults.coding.iter().cloned());
    }
    if let Some(style) = &selection.style {
        parts.push(style.instructions.clone());
    }
    parts
}

crate::registry::exports! {
    "format/output-style-system" {
        "outputStyleSystem" => ok |selection: StyleSelection, defaults: PromptDefaults|
            output_style_system(&selection, &defaults);
    }
}
