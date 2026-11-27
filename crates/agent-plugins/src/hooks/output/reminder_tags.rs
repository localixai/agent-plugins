//! Port of `src/hooks/output/reminder-tags.ts`, switched.
//!
//! hooks.md, "JSON output": "If your string contains a `<system-reminder>` or `</system-reminder>`
//! tag, Claude receives the string with that tag's `<` replaced by `&lt;`." The context of a
//! foreground hook and of a background one go through this one replacement.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::Pattern;

/// `/<(\/?system-reminder>)/g`.
static SYSTEM_REMINDER_TAG: Pattern = LazyLock::new(|| Regex::new(r"<(/?system-reminder>)").ok());

/// The text with the `<` of every system-reminder tag replaced by `&lt;`.
#[must_use]
pub fn escape_reminder_tags(text: &str) -> String {
    SYSTEM_REMINDER_TAG.as_ref().map_or_else(
        || text.to_owned(),
        |tag| tag.replace_all(text, "&lt;${1}").into_owned(),
    )
}

crate::registry::exports! {
    "hooks/output/reminder-tags" {
        "escapeReminderTags" => ok |text: String| escape_reminder_tags(&text);
    }
}
