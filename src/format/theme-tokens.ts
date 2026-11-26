/**
 * The names a theme file is made of (terminal-config.md, "Create a custom theme"): the six built-in
 * presets `base` takes, the 16 ANSI colour names, and the tokens `overrides` takes.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/theme_tokens.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/theme-tokens');

/** "Built-in preset the theme starts from" (the `base` row). The SDK's `theme` setting lists the same six, plus `auto`. */
export const THEME_BASES: readonly [
  'dark',
  'light',
  'dark-daltonized',
  'light-daltonized',
  'dark-ansi',
  'light-ansi',
] = call('THEME_BASES');
export type ThemeBase = (typeof THEME_BASES)[number];

/**
 * The 16 standard ANSI colour names `ansi:<name>` takes. The docs name two ("such as `red` or
 * `cyanBright`"); the rest follow the same scheme: eight colours and their `Bright` forms
 * (spec/judgments.md).
 */
export const ANSI_COLOR_NAMES: readonly [
  'black',
  'red',
  'green',
  'yellow',
  'blue',
  'magenta',
  'cyan',
  'white',
  'blackBright',
  'redBright',
  'greenBright',
  'yellowBright',
  'blueBright',
  'magentaBright',
  'cyanBright',
  'whiteBright',
] = call('ANSI_COLOR_NAMES');
export type AnsiColorName = (typeof ANSI_COLOR_NAMES)[number];

/**
 * The tokens of the "Color token reference" tables, the paired shimmer variants, the eight
 * `<color>_FOR_SUBAGENTS_ONLY` names and the `rainbow_<color>` (and `_shimmer`) names. The docs say
 * the list omits "a few single-purpose accents such as onboarding screen colors" (judgment: those
 * are kept as `extraOverrides` and listed in `unknownTokens`, never reported as a problem).
 */
export const THEME_TOKEN_NAMES: readonly string[] = call('THEME_TOKEN_NAMES');
