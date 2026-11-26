/**
 * A custom theme file (terminal-config.md, "Create a custom theme"): `~/.claude/themes/<slug>.json`,
 * and `themes/<slug>.json` in a plugin (plugins/components.md, "Themes and output styles"). Three
 * optional fields: `name`, `base` and `overrides`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/theme.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { AnsiColorName, ThemeBase } from './theme-tokens.ts';

const call = coreModule('format/theme');

/** A theme colour, as written: terminal palettes stay palette references (`ansi:` follows the terminal's own colours). */
export type ThemeColor =
  | { readonly form: 'rgb'; readonly r: number; readonly g: number; readonly b: number }
  | { readonly form: 'ansi256'; readonly index: number }
  | { readonly form: 'ansi'; readonly name: AnsiColorName };

export interface Theme {
  /** The file name without `.json`. */
  readonly slug: string;
  /** The label `/theme` shows: `name`, else the slug. */
  readonly name: string;
  readonly base: ThemeBase;
  /** The tokens set to a valid colour; every token not here falls through to the base preset. */
  readonly overrides: Readonly<Record<string, ThemeColor>>;
  /** Names in `overrides` that are not a documented token: ignored, in file order. */
  readonly unknownTokens: readonly string[];
  /**
   * Those entries as written, name to raw value. `/theme` sets "a few single-purpose accents such as
   * onboarding screen colors" that the reference omits: a copy or a rewrite writes them back
   * (judgment), so no valid token is lost.
   */
  readonly extraOverrides: Readonly<Record<string, unknown>>;
}

export interface ThemeRead {
  /** `undefined` when the file is not a JSON object. */
  readonly theme: Theme | undefined;
  readonly issues: readonly PluginIssue[];
}

/** The slug of a theme file: its name without `.json`; both `/` and `\` separate a path. */
export const themeSlug = (path: string): string => call('themeSlug', path);

/**
 * What `/theme` stores as the preference when the theme is selected: `custom:<slug>` for a theme of
 * `~/.claude/themes/`, `custom:<plugin-name>:<slug>` for one a plugin ships (settings-reference.md,
 * `theme`).
 */
export const customThemeId = (slug: string, plugin?: string): string =>
  call('customThemeId', slug, plugin);

/** A colour value as the docs accept it, or `undefined`: `#rrggbb`, `#rgb`, `rgb(r,g,b)`, `ansi256(n)`, `ansi:<name>`. */
export const readThemeColor = (value: unknown): ThemeColor | undefined =>
  call('readThemeColor', value);

/**
 * Read a theme file. Never throws. `name` defaults to the slug, `base` to `dark`; an invalid colour
 * and an unknown token are ignored, "so a typo cannot break rendering". What the docs leave open is in
 * spec/judgments.md: a base that is not a preset falls back to `dark` with a warning, an invalid colour
 * is a warning, and a file that is not a JSON object is not a theme (one error).
 */
export const readTheme = (slug: string, text: string): ThemeRead => call('readTheme', slug, text);
