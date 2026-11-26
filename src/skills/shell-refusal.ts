import type { SkillShell } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The commands the shell tools refuse to run (errors.md, "No shell command in a skill or command
 * was run"). Judgment (spec/judgments.md): a file with CRLF line endings is not refused for its
 * carriage returns, and the check comes before the permission check.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/shell_refusal.rs`, docs/RUST.md).
 */
const call = coreModule('skills/shell-refusal');

/** The message of errors.md for a command the tool does not accept, or `undefined` when it does. */
export const refusalOf = (command: string, name: string, shell: SkillShell): string | undefined =>
  call('refusalOf', command, name, shell);
