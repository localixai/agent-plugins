import type { PermissionBehavior } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ShellCommand, ShellWord } from './words.ts';

/**
 * The wrappers permissions.md ("Bash > Wrappers") says Claude Code strips before matching:
 * `timeout`, `time`, `nice`, `nohup` and `stdbuf`, the shell builtins `command` and `builtin`, zsh's
 * `noglob`, and bare `xargs`. "This wrapper list is built in and is not configurable."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/wrappers.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/wrappers');

/**
 * Where each form of a command starts among its words: at its first word, then after each wrapper
 * or assignment is removed in turn, so `timeout 5 nohup npm test` is also `nohup npm test` and `npm
 * test`.
 */
export const formStarts = (
  words: readonly ShellWord[],
  behavior: PermissionBehavior,
  safeEnv?: ReadonlySet<string>,
): readonly number[] => call('formStarts', words, behavior, safeEnv);

/** The words of the command a subcommand runs once every leading wrapper and assignment is removed. */
export const strippedWords = (
  command: ShellCommand,
  behavior: PermissionBehavior,
  safeEnv?: ReadonlySet<string>,
): readonly ShellWord[] => call('strippedWords', command, behavior, safeEnv);

/** The command a subcommand runs once every leading wrapper and assignment is removed, as written. */
export const stripWrappers = (
  subcommand: string,
  behavior: PermissionBehavior,
  safeEnv?: ReadonlySet<string>,
): string => call('stripWrappers', subcommand, behavior, safeEnv);

/**
 * The words of the command the read-only set judges once the wrappers it sees through are removed
 * (`READ_ONLY_STAGES`), and whether any was: `git` and `cd` behind one are not read-only.
 */
export const readOnlyWords = (
  words: readonly ShellWord[],
): {
  readonly words: readonly ShellWord[];
  readonly stripped: boolean;
} => call('readOnlyWords', words);
