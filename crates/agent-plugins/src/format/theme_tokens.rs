//! Port of `src/format/theme-tokens.ts`, switched.
//!
//! The names a theme file is made of (terminal-config.md, "Create a custom theme"): the six
//! built-in presets `base` takes, the 16 ANSI colour names, and the tokens `overrides` takes.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// One of the six built-in presets `base` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeBase {
    /// `dark`.
    Dark,
    /// `light`.
    Light,
    /// `dark-daltonized`.
    DarkDaltonized,
    /// `light-daltonized`.
    LightDaltonized,
    /// `dark-ansi`.
    DarkAnsi,
    /// `light-ansi`.
    LightAnsi,
}

impl ThemeBase {
    /// The preset's name as a theme file writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::DarkDaltonized => "dark-daltonized",
            Self::LightDaltonized => "light-daltonized",
            Self::DarkAnsi => "dark-ansi",
            Self::LightAnsi => "light-ansi",
        }
    }
}

/// "Built-in preset the theme starts from" (the `base` row). The SDK's `theme` setting lists the
/// same six, plus `auto`.
pub const THEME_BASES: [ThemeBase; 6] = [
    ThemeBase::Dark,
    ThemeBase::Light,
    ThemeBase::DarkDaltonized,
    ThemeBase::LightDaltonized,
    ThemeBase::DarkAnsi,
    ThemeBase::LightAnsi,
];

/// "Defaults to `dark`".
pub const DEFAULT_THEME_BASE: ThemeBase = ThemeBase::Dark;

/// One of the 16 standard ANSI colour names `ansi:<name>` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnsiColorName {
    /// `black`.
    Black,
    /// `red`.
    Red,
    /// `green`.
    Green,
    /// `yellow`.
    Yellow,
    /// `blue`.
    Blue,
    /// `magenta`.
    Magenta,
    /// `cyan`.
    Cyan,
    /// `white`.
    White,
    /// `blackBright`.
    BlackBright,
    /// `redBright`.
    RedBright,
    /// `greenBright`.
    GreenBright,
    /// `yellowBright`.
    YellowBright,
    /// `blueBright`.
    BlueBright,
    /// `magentaBright`.
    MagentaBright,
    /// `cyanBright`.
    CyanBright,
    /// `whiteBright`.
    WhiteBright,
}

impl AnsiColorName {
    /// The name as a theme file writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BlackBright => "blackBright",
            Self::RedBright => "redBright",
            Self::GreenBright => "greenBright",
            Self::YellowBright => "yellowBright",
            Self::BlueBright => "blueBright",
            Self::MagentaBright => "magentaBright",
            Self::CyanBright => "cyanBright",
            Self::WhiteBright => "whiteBright",
        }
    }
}

/// The 16 standard ANSI colour names `ansi:<name>` takes. The docs name two ("such as `red` or
/// `cyanBright`"); the rest follow the same scheme: eight colours and their `Bright` forms
/// (spec/judgments.md).
pub const ANSI_COLOR_NAMES: [AnsiColorName; 16] = [
    AnsiColorName::Black,
    AnsiColorName::Red,
    AnsiColorName::Green,
    AnsiColorName::Yellow,
    AnsiColorName::Blue,
    AnsiColorName::Magenta,
    AnsiColorName::Cyan,
    AnsiColorName::White,
    AnsiColorName::BlackBright,
    AnsiColorName::RedBright,
    AnsiColorName::GreenBright,
    AnsiColorName::YellowBright,
    AnsiColorName::BlueBright,
    AnsiColorName::MagentaBright,
    AnsiColorName::CyanBright,
    AnsiColorName::WhiteBright,
];

const SUBAGENT_COLORS: [&str; 8] = [
    "red", "blue", "green", "yellow", "purple", "orange", "pink", "cyan",
];
const RAINBOW_COLORS: [&str; 7] = [
    "red", "orange", "yellow", "green", "blue", "indigo", "violet",
];

/// The tokens of the "Color token reference" tables, the paired shimmer variants, the eight
/// `<color>_FOR_SUBAGENTS_ONLY` names and the `rainbow_<color>` (and `_shimmer`) names. The docs
/// say the list omits "a few single-purpose accents such as onboarding screen colors" (judgment:
/// those are kept as `extraOverrides` and listed in `unknownTokens`, never reported as a
/// problem).
const DOCUMENTED_TOKENS: [&str; 41] = [
    "claude",
    "text",
    "inverseText",
    "inactive",
    "subtle",
    "suggestion",
    "permission",
    "remember",
    "success",
    "error",
    "warning",
    "merged",
    "promptBorder",
    "planMode",
    "autoAccept",
    "bashBorder",
    "ide",
    "fastMode",
    "effortUltra",
    "diffAdded",
    "diffRemoved",
    "diffAddedDimmed",
    "diffRemovedDimmed",
    "diffAddedWord",
    "diffRemovedWord",
    "userMessageBackground",
    "userMessageBackgroundHover",
    "bashMessageBackgroundColor",
    "memoryBackgroundColor",
    "selectionBg",
    "composerSidebarBackground",
    "rate_limit_fill",
    "rate_limit_empty",
    "briefLabelYou",
    "briefLabelClaude",
    "claudeShimmer",
    "warningShimmer",
    "permissionShimmer",
    "promptBorderShimmer",
    "inactiveShimmer",
    "fastModeShimmer",
];

/// Every token name `overrides` takes.
pub static THEME_TOKEN_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut names: Vec<String> = DOCUMENTED_TOKENS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    names.extend(
        SUBAGENT_COLORS
            .iter()
            .map(|color| format!("{color}_FOR_SUBAGENTS_ONLY")),
    );
    for color in RAINBOW_COLORS {
        names.push(format!("rainbow_{color}"));
        names.push(format!("rainbow_{color}_shimmer"));
    }
    names
});

crate::registry::exports! {
    "format/theme-tokens" {
        "THEME_BASES" => ok || THEME_BASES;
        "ANSI_COLOR_NAMES" => ok || ANSI_COLOR_NAMES;
        "THEME_TOKEN_NAMES" => ok || &*THEME_TOKEN_NAMES;
    }
}
