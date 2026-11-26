import { coreModule } from '../wasm/bridge.ts';
import type { Theme } from './theme.ts';

/**
 * Writing a theme file back (plugins/components.md, "Themes and output styles": a plugin theme's edit is saved as a copy).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/theme_write.rs`, docs/RUST.md).
 */
const call = coreModule('format/theme-write');

export interface ThemeCopy {
  /** The slug the copy is saved under in the user's own themes directory (`~/.claude/themes/<slug>.json`). */
  readonly slug: string;
  readonly text: string;
}

/**
 * "Plugin themes are read-only, so when a user edits one in `/theme`, the edit is saved as a copy in
 * their own themes directory" (plugins/components.md, "Themes and output styles"). The copy keeps the
 * plugin theme's slug unless the user already has a theme under it, then `<slug>-copy`, `<slug>-copy-2`
 * and so on (spec/judgments.md: the docs do not name the copy).
 */
export const copyOfPluginTheme = (edited: Theme, taken: ReadonlySet<string>): ThemeCopy =>
  call('copyOfPluginTheme', edited, taken);
