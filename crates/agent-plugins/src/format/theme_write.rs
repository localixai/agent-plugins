//! Port of `src/format/theme-write.ts`, switched.
//!
//! Writing a theme file back (plugins/components.md, "Themes and output styles": a plugin theme's edit is saved as a copy).

use serde::Serialize;

use super::theme::{Theme, ThemeColor};
use crate::js::json_stringify_indented;
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::registry::Set;

fn hex(channel: u8) -> String {
    format!("{channel:02x}")
}

/// A colour written the way a theme file takes it: `#rrggbb`, `ansi256(n)` or `ansi:<name>`.
fn format_theme_color(color: ThemeColor) -> String {
    match color {
        ThemeColor::Rgb { r, g, b } => format!("#{}{}{}", hex(r), hex(g), hex(b)),
        ThemeColor::Ansi256 { index } => format!("ansi256({index})"),
        ThemeColor::Ansi { name } => format!("ansi:{}", name.as_str()),
    }
}

/// The file text of a theme: `name`, `base` and `overrides`, two-space JSON. The overrides are the
/// colours the kit read, then the tokens it does not list as they were written (`extraOverrides`).
fn theme_file_text(theme: &Theme) -> String {
    let mut overrides = JsObject::new();
    for (token, color) in &theme.overrides {
        overrides.insert(token.clone(), JsValue::String(format_theme_color(*color)));
    }
    for (token, value) in property_order(&theme.extra_overrides) {
        overrides.insert(token.clone(), value.clone());
    }
    let file = JsValue::Object(JsObject::from([
        ("name".to_owned(), JsValue::String(theme.name.clone())),
        (
            "base".to_owned(),
            JsValue::String(theme.base.as_str().to_owned()),
        ),
        (
            "overrides".to_owned(),
            JsValue::Object(in_property_order(overrides)),
        ),
    ]));
    format!(
        "{}\n",
        json_stringify_indented(&file, 2).unwrap_or_default()
    )
}

/// A theme saved as a copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ThemeCopy {
    /// The slug the copy is saved under in the user's own themes directory (`~/.claude/themes/<slug>.json`).
    pub slug: String,
    /// The file text.
    pub text: String,
}

/// "Plugin themes are read-only, so when a user edits one in `/theme`, the edit is saved as a copy in
/// their own themes directory" (plugins/components.md, "Themes and output styles"). The copy keeps the
/// plugin theme's slug unless the user already has a theme under it, then `<slug>-copy`, `<slug>-copy-2`
/// and so on (spec/judgments.md: the docs do not name the copy).
#[must_use]
pub fn copy_of_plugin_theme(edited: &Theme, taken: &[String]) -> ThemeCopy {
    let candidate = |n: usize| -> String {
        match n {
            0 => edited.slug.clone(),
            1 => format!("{}-copy", edited.slug),
            _ => format!("{}-copy-{n}", edited.slug),
        }
    };
    let mut attempt = 0;
    while taken.contains(&candidate(attempt)) {
        attempt += 1;
    }
    let slug = candidate(attempt);
    let copy = Theme {
        slug: slug.clone(),
        ..edited.clone()
    };
    ThemeCopy {
        slug,
        text: theme_file_text(&copy),
    }
}

crate::registry::exports! {
    "format/theme-write" {
        "copyOfPluginTheme" => ok |edited: Theme, taken: Set<String>| copy_of_plugin_theme(&edited, &taken.members);
    }
}
