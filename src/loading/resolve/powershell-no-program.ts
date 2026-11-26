import type { PluginIssue } from '../../format/issues.ts';
import type { ShellKind } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * hooks.md, "PowerShell started no program": "A hook with this error ran in PowerShell, which prints a
 * `command` that is only quoted strings, such as `"command": "\"C:\\Tools\\guard.exe\""`, instead of
 * running it." The fixes: `"shell": "powershell"` "to print text, such as JSON, and the hook exits 0";
 * `"shell": "powershell"` and `&` before the quoted path to run a script or a `.exe` whose path has a
 * variable; no inner quotes for one `.exe` at a path without `$`; exec form (`command` and `args`) or
 * Git Bash for a script written for Bash.
 *
 * Judgment (spec/judgments.md): the docs give the error's name and fixes, not when the kit may say it.
 * It says it for a shell-form hook that runs in PowerShell by the host's default (no `shell` field)
 * whose command is only quoted strings; "set `shell`: `powershell`" is the fix that makes the hook
 * exit 0, so a hook that names the shell is the author's choice and runs as written. A string is
 * `"..."` (a backtick or `""` inside is an escape) or `'...'` (`''` inside is one quote); several
 * strings separated by white space are still only strings.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/powershell_no_program.rs`, docs/RUST.md).
 */
const call = coreModule('loading/resolve/powershell-no-program');

export const onlyQuotedStrings = (command: string): boolean => call('onlyQuotedStrings', command);

/** What the hook's error says: the docs' sentence first, then the fixes. */
export const NO_PROGRAM: string = call('NO_PROGRAM');

/**
 * The hook does not start: it runs in PowerShell (`shell`) because the host defaults to it
 * (`named` is the handler's own `shell` field, absent) and the command is only quoted strings.
 */
export const noProgramIssue = (
  shell: ShellKind,
  named: ShellKind | undefined,
  path: string,
  command: string,
): PluginIssue | undefined => call('noProgramIssue', shell, named, path, command);
