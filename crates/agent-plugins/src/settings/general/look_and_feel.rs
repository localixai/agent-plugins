//! Port of `src/settings/general/look-and-feel.ts` at sha256:32020814efe2.
//!
//! The interface keys that pick one value from a closed set: `theme` (settings-reference.md,
//! `theme`; terminal-config.md, "Create a custom theme") and `teammateMode` (`teammateMode`), plus
//! `outputStyle`. Each reads the highest file's value; a value Claude Code does not recognize never
//! reaches the layer (the read drops it with a warning), so the next lower file or the default
//! applies. Takes the host's `PolicyView`: stays TypeScript in the package (the module comment of
//! `crate::settings`), so only its data names are exported.

use serde::{Deserialize, Serialize};

use crate::settings::view::PolicyView;

/// The theme presets.
pub const THEME_PRESETS: [&str; 7] = [
    "auto",
    "dark",
    "light",
    "dark-daltonized",
    "light-daltonized",
    "dark-ansi",
    "light-ansi",
];

/// A custom theme: the file name without `.json`, and the plugin that ships it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTheme {
    /// The plugin that ships it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// The file name without `.json`.
    pub slug: String,
}

/// The theme a session shows: a preset, or a custom one of `~/.claude/themes/` or of a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeChoice {
    /// One of `THEME_PRESETS`.
    Preset {
        /// Its name.
        preset: String,
    },
    /// A custom theme.
    Custom {
        /// Which one.
        custom: CustomTheme,
    },
}

fn default_theme() -> ThemeChoice {
    ThemeChoice::Preset {
        preset: "dark".to_owned(),
    }
}

fn is_preset(value: &str) -> bool {
    THEME_PRESETS.contains(&value)
}

/// `"custom:<slug>"` or `"custom:<plugin-name>:<slug>"`; `undefined` for any other text. A slug is
/// the file name without `.json`, so it is never empty (the docs name no further rule).
#[must_use]
pub fn parse_theme(value: &str) -> Option<ThemeChoice> {
    if is_preset(value) {
        return Some(ThemeChoice::Preset {
            preset: value.to_owned(),
        });
    }
    let mut pieces = value.split(':');
    let prefix = pieces.next();
    let parts: Vec<&str> = pieces.collect();
    if prefix != Some("custom") || parts.is_empty() || parts.len() > 2 || parts.contains(&"") {
        return None;
    }
    let first = parts.first()?;
    Some(match parts.get(1) {
        None => ThemeChoice::Custom {
            custom: CustomTheme {
                plugin: None,
                slug: (*first).to_owned(),
            },
        },
        Some(second) => ThemeChoice::Custom {
            custom: CustomTheme {
                plugin: Some((*first).to_owned()),
                slug: (*second).to_owned(),
            },
        },
    })
}

/// The theme: the highest file that sets it, else `legacy` (a value "in `~/.claude.json` from an
/// older version applies when no settings file sets it"), else `"dark"`.
#[must_use]
pub fn read_theme_setting(view: &dyn PolicyView, legacy: Option<&str>) -> ThemeChoice {
    if let Some(set) = view.get("theme") {
        return set
            .as_str()
            .and_then(parse_theme)
            .unwrap_or_else(default_theme);
    }
    legacy.and_then(parse_theme).unwrap_or_else(default_theme)
}

/// The teammate display modes.
pub const TEAMMATE_MODES: [&str; 4] = ["in-process", "auto", "tmux", "iterm2"];

fn is_teammate_mode(value: Option<&str>) -> bool {
    value.is_some_and(|mode| TEAMMATE_MODES.contains(&mode))
}

/// The teammate display mode: a file's value, else `legacy` from `~/.claude.json`, else
/// `"in-process"`. A file's value the layer admits is one of `TEAMMATE_MODES`; any other falls to
/// the next rule here, where the package would pass it on.
#[must_use]
pub fn read_teammate_mode(view: &dyn PolicyView, legacy: Option<&str>) -> String {
    if let Some(set) = view.get("teammateMode")
        && let Some(mode) = set.as_str().filter(|mode| is_teammate_mode(Some(mode)))
    {
        return mode.to_owned();
    }
    match legacy {
        Some(mode) if is_teammate_mode(Some(mode)) => mode.to_owned(),
        Some(_) | None => "in-process".to_owned(),
    }
}

/// The output style named by the highest file; `undefined` selects the default style.
#[must_use]
pub fn read_output_style_setting(view: &dyn PolicyView) -> Option<String> {
    view.get("outputStyle")
        .and_then(|value| value.as_str().map(str::to_owned))
}

crate::registry::exports! {
    "settings/general/look-and-feel" {
        "THEME_PRESETS" => ok || THEME_PRESETS;
        "parseTheme" => ok |value: String| parse_theme(&value);
        "TEAMMATE_MODES" => ok || TEAMMATE_MODES;
    }
}
