//! Port of `src/format/theme.ts`, switched.
//!
//! A custom theme file (terminal-config.md, "Create a custom theme"):
//! `~/.claude/themes/<slug>.json`, and `themes/<slug>.json` in a plugin (plugins/components.md,
//! "Themes and output styles"). Three optional fields: `name`, `base` and `overrides`.

use crate::js::{JsObject, JsValue};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error, warning};
use super::theme_tokens::{
    ANSI_COLOR_NAMES, AnsiColorName, DEFAULT_THEME_BASE, THEME_BASES, THEME_TOKEN_NAMES, ThemeBase,
};
use crate::js::{property_order, trim};

/// A theme colour, as written: terminal palettes stay palette references (`ansi:` follows the
/// terminal's own colours).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "form", rename_all = "lowercase")]
pub enum ThemeColor {
    /// `#rrggbb`, `#rgb` or `rgb(r,g,b)`.
    Rgb {
        /// Red, 0 to 255.
        r: u8,
        /// Green, 0 to 255.
        g: u8,
        /// Blue, 0 to 255.
        b: u8,
    },
    /// `ansi256(n)`.
    Ansi256 {
        /// The palette index, 0 to 255.
        index: u8,
    },
    /// `ansi:<name>`.
    Ansi {
        /// One of the 16 ANSI colour names.
        name: AnsiColorName,
    },
}

fn is_token(name: &str) -> bool {
    THEME_TOKEN_NAMES.iter().any(|token| token == name)
}

fn ansi_name(name: &str) -> Option<AnsiColorName> {
    ANSI_COLOR_NAMES
        .iter()
        .copied()
        .find(|each| each.as_str() == name)
}

/// A theme file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// The file name without `.json`.
    pub slug: String,
    /// The label `/theme` shows: `name`, else the slug.
    pub name: String,
    /// The preset it starts from.
    pub base: ThemeBase,
    /// The tokens set to a valid colour; every token not here falls through to the base preset.
    pub overrides: IndexMap<String, ThemeColor>,
    /// Names in `overrides` that are not a documented token: ignored, in file order.
    pub unknown_tokens: Vec<String>,
    /// Those entries as written, name to raw value. `/theme` sets "a few single-purpose accents
    /// such as onboarding screen colors" that the reference omits: a copy or a rewrite writes
    /// them back (judgment), so no valid token is lost.
    pub extra_overrides: IndexMap<String, JsValue>,
}

/// What reading a theme file gave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeRead {
    /// `None` when the file is not a JSON object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// What was wrong with it.
    pub issues: Vec<PluginIssue>,
}

/// The slug of a theme file: its name without `.json`; both `/` and `\` separate a path.
#[must_use]
pub fn theme_slug(path: &str) -> String {
    let base = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let cut = base.len().saturating_sub(".json".len());
    match base.get(cut..) {
        Some(tail) if tail.eq_ignore_ascii_case(".json") => base.get(..cut).unwrap_or(base),
        Some(_) | None => base,
    }
    .to_owned()
}

/// What `/theme` stores as the preference when the theme is selected: `custom:<slug>` for a theme
/// of `~/.claude/themes/`, `custom:<plugin-name>:<slug>` for one a plugin ships
/// (settings-reference.md, `theme`).
#[must_use]
pub fn custom_theme_id(slug: &str, plugin: Option<&str>) -> String {
    match plugin {
        None => format!("custom:{slug}"),
        Some(plugin) => format!("custom:{plugin}:{slug}"),
    }
}

/// The number a run of one to three ASCII digits writes (`\d{1,3}`), blanks around it allowed.
fn channel(text: &str) -> Option<u16> {
    let digits = trim(text);
    if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn hex_pair(text: &str) -> Option<u8> {
    u8::from_str_radix(text, 16).ok()
}

/// The text of the form `<name>(<inside>)`, with the inside.
fn call<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
}

type ColorParser = fn(&str) -> Option<ThemeColor>;

/// `#rrggbb`.
fn hex_color(text: &str) -> Option<ThemeColor> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(ThemeColor::Rgb {
        r: hex_pair(digits.get(0..2)?)?,
        g: hex_pair(digits.get(2..4)?)?,
        b: hex_pair(digits.get(4..6)?)?,
    })
}

/// `#rgb`.
fn short_hex_color(text: &str) -> Option<ThemeColor> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 3 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let double = |digit: Option<&str>| hex_pair(&format!("{0}{0}", digit?));
    Some(ThemeColor::Rgb {
        r: double(digits.get(0..1))?,
        g: double(digits.get(1..2))?,
        b: double(digits.get(2..3))?,
    })
}

/// `rgb(r, g, b)`.
fn rgb_call(text: &str) -> Option<ThemeColor> {
    let parts: Vec<u8> = call(text, "rgb")?
        .split(',')
        .map(|part| channel(part).and_then(|value| u8::try_from(value).ok()))
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [r, g, b] => Some(ThemeColor::Rgb {
            r: *r,
            g: *g,
            b: *b,
        }),
        _ => None,
    }
}

/// `ansi256(n)`.
fn ansi256_call(text: &str) -> Option<ThemeColor> {
    let index = u8::try_from(channel(call(text, "ansi256")?)?).ok()?;
    Some(ThemeColor::Ansi256 { index })
}

/// `ansi:<name>`.
fn ansi_color(text: &str) -> Option<ThemeColor> {
    let named = text.strip_prefix("ansi:")?;
    if named.is_empty()
        || !named
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    ansi_name(named).map(|name| ThemeColor::Ansi { name })
}

/// One parser per form the docs list. Whitespace is allowed inside the parentheses (judgment).
const COLOR_FORMS: [ColorParser; 5] = [
    hex_color,
    short_hex_color,
    rgb_call,
    ansi256_call,
    ansi_color,
];

/// A colour value as the docs accept it, or `None`: `#rrggbb`, `#rgb`, `rgb(r,g,b)`,
/// `ansi256(n)`, `ansi:<name>`.
#[must_use]
pub fn read_theme_color(value: &JsValue) -> Option<ThemeColor> {
    let text = trim(value.as_str()?);
    COLOR_FORMS.iter().find_map(|form| form(text))
}

fn is_base(value: &JsValue) -> Option<ThemeBase> {
    let name = value.as_str()?;
    THEME_BASES
        .iter()
        .copied()
        .find(|each| each.as_str() == name)
}

struct Overrides {
    overrides: IndexMap<String, ThemeColor>,
    extra: IndexMap<String, JsValue>,
    issues: Vec<PluginIssue>,
}

/// Keeps the documented tokens with a valid colour; every other entry is ignored, as the docs say.
fn read_overrides(raw: Option<&JsValue>) -> Overrides {
    let Some(raw) = raw else {
        return Overrides {
            overrides: IndexMap::new(),
            extra: IndexMap::new(),
            issues: Vec::new(),
        };
    };
    let Some(members) = raw.as_object() else {
        let issue = warning(
            "overrides",
            "overrides must be an object of token names to colors; ignored",
        );
        return Overrides {
            overrides: IndexMap::new(),
            extra: IndexMap::new(),
            issues: vec![issue],
        };
    };
    let mut kept = IndexMap::new();
    let mut extra = IndexMap::new();
    let mut issues = Vec::new();
    for (token, value) in property_order(members) {
        if !is_token(token) {
            extra.insert(token.clone(), value.clone());
            continue;
        }
        match read_theme_color(value) {
            Some(color) => {
                kept.insert(token.clone(), color);
            }
            None => issues.push(warning(
                &format!("overrides.{token}"),
                &format!("{token} is not a color Claude Code accepts; ignored"),
            )),
        }
    }
    Overrides {
        overrides: kept,
        extra,
        issues,
    }
}

fn parse_object(text: &str) -> Option<JsObject> {
    match crate::js::json_parse::json_parse(text) {
        Ok(JsValue::Object(members)) => Some(members),
        Ok(_) | Err(_) => None,
    }
}

/// Read a theme file. Never throws. `name` defaults to the slug, `base` to `dark`; an invalid
/// colour and an unknown token are ignored, "so a typo cannot break rendering". What the docs
/// leave open is in spec/judgments.md: a base that is not a preset falls back to `dark` with a
/// warning, an invalid colour is a warning, and a file that is not a JSON object is not a theme
/// (one error).
#[must_use]
pub fn read_theme(slug: &str, text: &str) -> ThemeRead {
    let Some(raw) = parse_object(text) else {
        return ThemeRead {
            theme: None,
            issues: vec![error("theme", &format!("{slug}.json is not a JSON object"))],
        };
    };
    let mut issues = Vec::new();
    let base = raw
        .get("base")
        .and_then(is_base)
        .unwrap_or(DEFAULT_THEME_BASE);
    if raw
        .get("base")
        .is_some_and(|value| is_base(value).is_none())
    {
        let names: Vec<&str> = THEME_BASES.iter().map(|each| each.as_str()).collect();
        issues.push(warning(
            "base",
            &format!(
                "base must be one of {}; using {}",
                names.join(", "),
                DEFAULT_THEME_BASE.as_str()
            ),
        ));
    }
    let label = raw.get("name").and_then(JsValue::as_str).map_or("", trim);
    let read = read_overrides(raw.get("overrides"));
    let theme = Theme {
        slug: slug.to_owned(),
        name: if label.is_empty() { slug } else { label }.to_owned(),
        base,
        overrides: read.overrides,
        unknown_tokens: read.extra.keys().cloned().collect(),
        extra_overrides: read.extra,
    };
    issues.extend(read.issues);
    ThemeRead {
        theme: Some(theme),
        issues,
    }
}

crate::registry::exports! {
    "format/theme" {
        "themeSlug" => ok |path: String| theme_slug(&path);
        "customThemeId" => ok |slug: String, plugin: Option<String>| custom_theme_id(&slug, plugin.as_deref());
        "readThemeColor" => ok |value: JsValue| read_theme_color(&value);
        "readTheme" => ok |slug: String, text: String| read_theme(&slug, &text);
    }
}
