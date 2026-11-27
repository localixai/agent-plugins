/**
 * How a `!` command receives the invocation's arguments (skills.md, "Available string
 * substitutions"; docs/SECURITY.md, "`$ARGUMENTS` in `!` blocks"). Judgment (spec/judgments.md,
 * `shellArguments`): `pasted`, the default, writes each value into the command as Claude Code does;
 * `environment-reference` gives a bash command each placeholder as a reference to a variable that
 * holds the value, so the shell never parses a value as code, whatever quoting the author put
 * around the placeholder. The permission check reads the command with each value quoted where it
 * lands. PowerShell keeps the values pasted.
 *
 * Types only: the core binds the arguments (`crates/agent-plugins/src/skills/shell_arguments.rs`,
 * docs/RUST.md), and only the core calls it now.
 */
export type ShellArguments = 'pasted' | 'environment-reference';
