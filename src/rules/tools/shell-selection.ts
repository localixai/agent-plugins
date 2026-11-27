/**
 * tools-reference.md, "Shell selection in settings, hooks, and skills": `"defaultShell": "powershell"`
 * "routes interactive `!` commands through PowerShell. Requires the PowerShell tool to be enabled."
 * (A command hook's `shell` and a skill's `shell` are read by `hooks` and `skills`.)
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/shell_selection.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/tools/shell-selection');

export type InteractiveShell = 'bash' | 'powershell';

/**
 * The shell an interactive `!` command runs in. Judgment (spec/judgments.md): any `defaultShell`
 * other than `powershell`, and `powershell` while the PowerShell tool is off, leaves Bash.
 */
export const interactiveShell = (input: {
  readonly defaultShell: string | undefined;
  readonly powerShellToolEnabled: boolean;
}): InteractiveShell => call('interactiveShell', input);
