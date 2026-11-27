/**
 * The interface keys that pick one value from a closed set: `theme` (settings-reference.md, `theme`;
 * terminal-config.md, "Create a custom theme") and `teammateMode` (`teammateMode`), plus `outputStyle`.
 * Each reads the highest file's value; a value Claude Code does not recognize never reaches the layer
 * (the read drops it with a warning), so the next lower file or the default applies.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/look_and_feel.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/general/look-and-feel');

export const THEME_PRESETS: readonly [
  'auto',
  'dark',
  'light',
  'dark-daltonized',
  'light-daltonized',
  'dark-ansi',
  'light-ansi',
] = call('THEME_PRESETS');
export type ThemePreset = (typeof THEME_PRESETS)[number];

/** The theme a session shows: a preset, or a custom one of `~/.claude/themes/` or of a plugin. */
export type ThemeChoice =
  | { readonly preset: ThemePreset }
  | { readonly custom: { readonly slug: string; readonly plugin?: string } };

/**
 * `"custom:<slug>"` or `"custom:<plugin-name>:<slug>"`; `undefined` for any other text. A slug is the
 * file name without `.json`, so it is never empty (the docs name no further rule).
 */
export const parseTheme = (value: string): ThemeChoice | undefined => call('parseTheme', value);

/**
 * The theme: the highest file that sets it, else `legacy` (a value "in `~/.claude.json` from an older
 * version applies when no settings file sets it"), else `"dark"`.
 */
export const readThemeSetting = (view: PolicyView, legacy?: string): ThemeChoice =>
  call('readThemeSetting', view, legacy);

export const TEAMMATE_MODES: readonly ['in-process', 'auto', 'tmux', 'iterm2'] =
  call('TEAMMATE_MODES');
export type TeammateMode = (typeof TEAMMATE_MODES)[number];

/** The teammate display mode: a file's value, else `legacy` from `~/.claude.json`, else `"in-process"`. */
export const readTeammateMode = (view: PolicyView, legacy?: string): TeammateMode =>
  call('readTeammateMode', view, legacy);

/** The output style named by the highest file; `undefined` selects the default style. */
export const readOutputStyleSetting = (view: PolicyView): string | undefined =>
  call('readOutputStyleSetting', view);
