import type { SkillShell } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ArgumentInput } from './substitute.ts';

const call = coreModule('skills/shell-arguments');

/**
 * How a `!` command receives the invocation's arguments (skills.md, "Available string
 * substitutions"; docs/SECURITY.md, "`$ARGUMENTS` in `!` blocks"). Judgment (spec/judgments.md,
 * `shellArguments`): `pasted`, the default, writes each value into the command as Claude Code does;
 * `environment-reference` gives a bash command each placeholder as a reference to a variable that
 * holds the value, so the shell never parses a value as code, whatever quoting the author put
 * around the placeholder. The permission check reads the command with each value quoted where it
 * lands. PowerShell keeps the values pasted.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/shell_arguments.rs`, docs/RUST.md).
 */
export type ShellArguments = 'pasted' | 'environment-reference';

/** A `!` command with its arguments bound: what is checked, what the shell runs, and its variables. */
export interface BoundArguments {
  readonly checked: string;
  readonly shell: string;
  readonly environment: Readonly<Record<string, string>>;
}

export const bindsArguments = (choice: ShellArguments | undefined, shell: SkillShell): boolean =>
  call('bindsArguments', choice, shell);

/**
 * `command`'s argument placeholders bound: an unquoted `$ARGUMENTS` is each argument as its own
 * word (the typed quoting groups words), in quotes the text as typed; `$N`, `$ARGUMENTS[N]` and a
 * declared `$name` are one argument. Escapes and unmatched placeholders are as `substituteArguments`
 * leaves them.
 */
export const bindShellArguments = (command: string, input: ArgumentInput): BoundArguments =>
  call('bindShellArguments', command, { rawArguments: input.rawArguments, names: input.names });
