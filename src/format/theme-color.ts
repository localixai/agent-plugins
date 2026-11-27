import { coreModule } from '../wasm/bridge.ts';
import type { ThemeColor } from './theme.ts';
import type { AnsiColorName } from './theme-tokens.ts';

/**
 * A theme colour as CSS (judgment: the docs give the forms of a colour, not what a host that draws
 * with CSS makes of them). `rgb` is exact. `ansi256` and `ansi:<name>` are palette references: "the
 * terminal's own colours", so no colour is fixed by the theme; the kit's default palette is xterm's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/theme_color.rs`, docs/RUST.md).
 */
const call = coreModule('format/theme-color');

/** The 16 ANSI colours as `#rrggbb`, by name: what a terminal draws for `ansi:<name>`. */
export type AnsiPalette = Readonly<Record<AnsiColorName, string>>;

/** xterm's default 16 colours: the eight, then their `Bright` forms. */
export const XTERM_ANSI_PALETTE: AnsiPalette = call('XTERM_ANSI_PALETTE');

/**
 * The colour as `#rrggbb`. `rgb` is itself; `ansi:<name>` is read from `palette`; `ansi256(n)` is the
 * palette for 0 to 15, xterm's 6x6x6 cube for 16 to 231 and its grey ramp for 232 to 255.
 */
export const themeColorToHex = (
  color: ThemeColor,
  palette: AnsiPalette = XTERM_ANSI_PALETTE,
): string => call('themeColorToHex', color, palette);
