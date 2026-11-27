import { coreModule } from '../../wasm/bridge.ts';
import type { ShellWord } from '../bash/words.ts';
import type { PathContext } from '../types.ts';

/**
 * The built-in read-only Bash commands (permissions.md, "Read-only commands") and what keeps each
 * one's arguments read-only. `cd` and `git` have their own rules in `shell-commands.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/read_only_set.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/read-only-set');

/**
 * "The set includes `ls`, `cat`, `echo`, `pwd`, `head`, `tail`, `grep`, `find`, `wc`, `which`,
 * `diff`, `stat`, `du`, `cd`, and read-only forms of `git`", and the docs' prompt cases name
 * `docker`, `file` and `ps` as members.
 */
export const READ_ONLY_PROGRAMS: ReadonlySet<string> = call('READ_ONLY_PROGRAMS');

/**
 * The "read-only forms of `git`" the docs do not list: a seed (spec/judgments.md), each subcommand
 * with what keeps its arguments read-only.
 */
export const GIT_READ_ONLY: {
  readonly [subcommand: string]: (args: readonly string[]) => boolean;
} = call('GIT_READ_ONLY');

/** Whether the arguments of `program` are read-only: a program with no entry is read-only whatever it is given. */
export const argumentsAreReadOnly = (program: string, words: readonly string[]): boolean =>
  call('argumentsAreReadOnly', program, words);

/** "a command whose arguments include a network (UNC) path ... prompts" (Windows only). */
export const hasNetworkArgument = (words: readonly ShellWord[], paths: PathContext): boolean =>
  call('hasNetworkArgument', words, paths);

/** Whether a command sets, unsets or loops over a special shell variable, anywhere in it (quoted text included). */
export const touchesSpecialVariable = (command: string): boolean =>
  call('touchesSpecialVariable', command);
