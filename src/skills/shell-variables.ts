import type { SkillShell } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SkillVariables } from './variables.ts';

/**
 * The text the shell receives for a skill's `!` command (skills.md, "Available string
 * substitutions"; "How injected commands run"). The permission check reads the same command as
 * skill content, its paths written in (`substituteVariables`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/shell_variables.rs`, docs/RUST.md).
 */

const call = coreModule('skills/shell-variables');

/** What the shell receives for one `!` command. */
export interface ShellCommandText {
  readonly text: string;
  /** The values of the references `text` holds, for the command's environment. */
  readonly environment: Readonly<Record<string, string>>;
}

/**
 * `${CLAUDE_*}` in a `!` command as its shell receives it: under the scope's `shellPaths`, each path
 * a reference the shell expands from the environment returned with it (judgment, spec/judgments.md,
 * `shellPaths`), or written in when the host asks for `pasted`.
 */
export const substituteShellCommand = (
  text: string,
  vars: SkillVariables,
  shell: SkillShell,
): Promise<ShellCommandText> => call.async('substituteShellCommand', text, vars, shell);
