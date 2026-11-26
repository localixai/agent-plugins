import { coreModule } from '../../../wasm/bridge.ts';

/**
 * tools-reference.md, "Output limits": "A command that exits 1 counts as a valid result for the
 * Bash tool only when Claude Code recognizes exit code 1 as a benign outcome for that command:
 * `grep`, `rg`, `egrep`, `fgrep`, `find`, `diff`, `test`, and `[`, plus `git diff` and `git grep`."
 * The PowerShell tool's list is its own (tools-reference.md, "Exit code 1 from searches and `git
 * diff` in PowerShell").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/bash_exit.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/tools/semantics/bash-exit');

/**
 * Whether Claude Code counts the finished command as a valid result rather than a failure: exit 0
 * always, exit 1 only for the commands above, anything else never. A compound line exits with its
 * last command, and a command behind a wrapper, a variable assignment, a `!`, a subshell or a
 * trailing `&`, or with options before the subcommand (`sudo grep`, `git -C . diff`), is read as
 * written, so it is a failure (spec/judgments.md).
 */
export const classifyBashExit = (command: string, exitCode: number): boolean =>
  call('classifyBashExit', command, exitCode);

/**
 * The same for the PowerShell tool, whose benign list is its own (`findstr` in, `find`, `diff`,
 * `test` and `[` out). A line is split as a Bash line is (spec/judgments.md).
 */
export const classifyPowerShellExit = (command: string, exitCode: number): boolean =>
  call('classifyPowerShellExit', command, exitCode);
