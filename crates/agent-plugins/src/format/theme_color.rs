//! Port of `src/format/theme-color.ts`, switched.
//!
//! A theme colour as CSS (judgment: the docs give the forms of a colour, not what a host that draws
//! with CSS makes of them). `rgb` is exact. `ansi256` and `ansi:<name>` are palette references: "the
//! terminal's own colours", so no colour is fixed by the theme; the kit's default palette is xterm's.

use std::sync::LazyLock;

use indexmap::IndexMap;

use super::theme::ThemeColor;
use super::theme_tokens::{ANSI_COLOR_NAMES, AnsiColorName};

/// xterm's default 16 colours: the eight, then their `Bright` forms.
const XTERM_ANSI_COLORS: [(AnsiColorName, &str); 16] = [
    (AnsiColorName::Black, "#000000"),
    (AnsiColorName::Red, "#cd0000"),
    (AnsiColorName::Green, "#00cd00"),
    (AnsiColorName::Yellow, "#cdcd00"),
    (AnsiColorName::Blue, "#0000ee"),
    (AnsiColorName::Magenta, "#cd00cd"),
    (AnsiColorName::Cyan, "#00cdcd"),
    (AnsiColorName::White, "#e5e5e5"),
    (AnsiColorName::BlackBright, "#7f7f7f"),
    (AnsiColorName::RedBright, "#ff0000"),
    (AnsiColorName::GreenBright, "#00ff00"),
    (AnsiColorName::YellowBright, "#ffff00"),
    (AnsiColorName::BlueBright, "#5c5cff"),
    (AnsiColorName::MagentaBright, "#ff00ff"),
    (AnsiColorName::CyanBright, "#00ffff"),
    (AnsiColorName::WhiteBright, "#ffffff"),
];

/// The 16 ANSI colours as `#rrggbb`, by name: what a terminal draws for `ansi:<name>`
/// (`AnsiPalette`); `XTERM_ANSI_PALETTE` is xterm's.
pub type AnsiPalette = IndexMap<String, String>;

/// xterm's default palette, by name (`XTERM_ANSI_PALETTE`).
pub static XTERM_ANSI_PALETTE: LazyLock<AnsiPalette> = LazyLock::new(|| {
    XTERM_ANSI_COLORS
        .iter()
        .map(|(name, color)| (name.as_str().to_owned(), (*color).to_owned()))
        .collect()
});

fn to_hex_channel(channel: u8) -> String {
    format!("{channel:02x}")
}

fn to_hex(r: u8, g: u8, b: u8) -> String {
    format!(
        "#{}{}{}",
        to_hex_channel(r),
        to_hex_channel(g),
        to_hex_channel(b)
    )
}

/// The six levels of each channel of the 6x6x6 colour cube (indexes 16 to 231), as xterm draws them.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
const CUBE_START: u8 = 16;
const GRAY_START: u8 = 232;

/// What `palette` draws for `name`; a palette that lacks it (the type says it has all) draws xterm's.
fn palette_color(palette: &AnsiPalette, name: AnsiColorName) -> String {
    palette.get(name.as_str()).cloned().unwrap_or_else(|| {
        XTERM_ANSI_COLORS
            .iter()
            .find(|(each, _)| *each == name)
            .map_or_else(String::new, |(_, color)| (*color).to_owned())
    })
}

fn ansi256_hex(index: u8, palette: &AnsiPalette) -> String {
    if let Some(named) = ANSI_COLOR_NAMES.get(usize::from(index)) {
        return palette_color(palette, *named);
    }
    if index >= GRAY_START {
        let level = 8 + (index - GRAY_START) * 10;
        return to_hex(level, level, level);
    }
    let cube = index - CUBE_START;
    let level = |at: u8| -> u8 { CUBE_LEVELS.get(usize::from(at % 6)).copied().unwrap_or(0) };
    to_hex(level(cube / 36), level(cube / 6), level(cube))
}

/// The colour as `#rrggbb`. `rgb` is itself; `ansi:<name>` is read from `palette`; `ansi256(n)` is the
/// palette for 0 to 15, xterm's 6x6x6 cube for 16 to 231 and its grey ramp for 232 to 255.
#[must_use]
pub fn theme_color_to_hex(color: &ThemeColor, palette: &AnsiPalette) -> String {
    match color {
        ThemeColor::Rgb { r, g, b } => to_hex(*r, *g, *b),
        ThemeColor::Ansi { name } => palette_color(palette, *name),
        ThemeColor::Ansi256 { index } => ansi256_hex(*index, palette),
    }
}

crate::registry::exports! {
    "format/theme-color" {
        "XTERM_ANSI_PALETTE" => ok || XTERM_ANSI_PALETTE.clone();
        "themeColorToHex" => ok |color: ThemeColor, palette: Option<AnsiPalette>| theme_color_to_hex(&color, &palette.unwrap_or_else(|| XTERM_ANSI_PALETTE.clone()));
    }
}
