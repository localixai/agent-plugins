/**
 * Whether the inline shell of a skill runs (settings-reference.md, `disableSkillShellExecution`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/skills/shell.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/skills/shell');

/** Where a skill or custom command comes from, for the one question inline shell asks. */
export const SHELL_ORIGINS: readonly ['user', 'project', 'plugin', 'addDir', 'bundled', 'managed'] =
  call('SHELL_ORIGINS');
export type ShellOrigin = (typeof SHELL_ORIGINS)[number];

/** The text that takes the place of an inline command (settings-reference.md, `disableSkillShellExecution`). */
export const SHELL_DISABLED_MARKER: '[shell command execution disabled by policy]' =
  call('SHELL_DISABLED_MARKER');

export type ShellVerdict =
  | { readonly run: true }
  | { readonly run: false; readonly replacement: typeof SHELL_DISABLED_MARKER };

/**
 * What the skills expansion does with the `` !`cmd` `` and ` ```! ` blocks of a skill from `origin`.
 * Replacing the blocks themselves is the skills expansion's; this is its verdict.
 */
export const shellVerdict = (view: PolicyView, origin: ShellOrigin): ShellVerdict =>
  call('shellVerdict', view, origin);
