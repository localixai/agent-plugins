import { coreModule } from '../../wasm/bridge.ts';
import type { ShellWord } from '../bash/words.ts';
import type { PathContext } from '../types.ts';
import type { ProtectedPathOptions } from './context.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/shell_words.rs`, docs/RUST.md). */
const call = coreModule('rules/decide/shell-words');

/** The state of the walk over a command's subcommands, in the order they run. */
export interface Walk {
  readonly paths: PathContext;
  readonly options: ProtectedPathOptions;
  /** Where the host's symlink check says the call's paths lead (`PreparedCall.resolvedPaths`). */
  readonly resolved: readonly string[];
  /** The directory later operands are read against: moved by a `cd`. */
  base: string;
  changedDirectory: boolean;
  usedGit: boolean;
}

export const hasUnquotedGlob = (words: readonly ShellWord[]): boolean =>
  call('hasUnquotedGlob', words);

/** The absolute POSIX path of an operand, or `undefined` when a variable, substitution or glob hides it. */
export const operandPath = (operand: ShellWord, walk: Walk): string | undefined =>
  call('operandPath', operand, walk);

/**
 * Where a file-reading operand reads: the path itself, the directory a glob starts in, or
 * `undefined` for an operand whose value a variable or substitution hides.
 */
export const readPath = (operand: ShellWord, walk: Walk): string | undefined =>
  call('readPath', operand, walk);
