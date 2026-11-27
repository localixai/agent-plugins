/**
 * The files a Bash command names, as the `Read` and `Edit` rules see them (permissions.md, "What
 * `Read` and `Edit` rules cover": the rules apply "to file commands Claude Code recognizes in Bash,
 * such as `cat`, `head`, `tail`, `sed`, and `tee`, and to the targets of Bash redirections such as
 * `> file` and `< file`"; "a command that reads files without naming them" is not covered). A
 * lexical reading of the line, not a sandbox; an operand a variable or substitution hides is left
 * out. Judgment (spec/judgments.md): the docs name five of the commands, so the set is the kit's,
 * and a `cd` earlier in the line does not move the directory later operands are read against.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/file_targets.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext } from '../types.ts';

const call = coreModule('rules/decide/file-targets');

export interface BashFileTargets {
  /** Absolute POSIX paths the command reads. */
  readonly reads: readonly string[];
  /** Absolute POSIX paths the command writes. */
  readonly writes: readonly string[];
}

/** The files a Bash command reads and writes by naming them, read against the starting directory. */
export const bashFileTargets = (command: string, paths: PathContext): BashFileTargets =>
  call('bashFileTargets', command, paths);
