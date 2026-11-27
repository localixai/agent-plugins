/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/split.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import { BASH_DIALECT, type ShellDialect } from './dialect.ts';

const call = coreModule('rules/bash/split');

/** The subcommands a command line runs, and whether the kit could read it all. */
export interface SplitCommand {
  /** In reading order, a command before the ones nested in it; best effort when not parseable. */
  readonly subcommands: readonly string[];
  /** False for a command with nothing after `&&` or `||`, or an unclosed quote or bracket. */
  readonly parseable: boolean;
}

/**
 * Splits a command line into the commands it runs (permissions.md, "Compound commands"): at every
 * separator outside quotes and comments, and again inside `$()`, backticks, subshells, function
 * bodies, unquoted here-document bodies and control-flow bodies, so a rule sees a command nested in
 * another. The text of each subcommand is as written, comments cut. A command the kit cannot read
 * to the end is `parseable: false`, with the subcommands found so far.
 */
export const splitCommand = (command: string, dialect: ShellDialect = BASH_DIALECT): SplitCommand =>
  call('splitCommand', command, dialect);
