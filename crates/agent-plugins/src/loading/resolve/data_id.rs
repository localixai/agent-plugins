//! Port of `src/loading/resolve/data-id.ts`, switched.
//!
//! The `<id>` of `~/.claude/plugins/data/<id>/` (plugins/manifest-reference.md, "Environment
//! variables"): the plugin identifier with every character other than a letter, digit, `_` or `-`
//! replaced by `-`. A character is a code point, so one outside the basic plane becomes one `-`.

/// The plugin's data folder name.
#[must_use]
pub fn plugin_data_id(identifier: &str) -> String {
    identifier
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

crate::registry::exports! {
    "loading/resolve/data-id" {
        "pluginDataId" => ok |identifier: String| plugin_data_id(&identifier);
    }
}
