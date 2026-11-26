/**
 * hooks.md, "Persisted variables in PowerShell commands": PowerShell never runs `CLAUDE_ENV_FILE`;
 * "Claude Code reads the assignments out of it and copies them into the PowerShell command's
 * environment instead", and "only when every line is one of the following":
 *
 * - a blank line or a `#` comment
 * - one assignment at the start of the line, `export NAME=value`, `declare -x NAME=value` or
 *   `NAME=value`, with a value "Bash would use exactly as written", made of any mix of unquoted text
 *   that uses only letters, digits and `_ @ % + = : , . / -`, text in single quotes, and text in
 *   double quotes where any `$`, backtick or `"` inside is escaped with a backslash
 *
 * "If any line is of another kind ... PowerShell commands receive none of the variables, and
 * `claude --debug` logs `Session environment is not all plain assignments`." On Windows "PowerShell
 * commands also don't receive a variable whose value contains `/` or `\`", and "a sandboxed
 * PowerShell command receives none of the variables".
 *
 * Judgments (spec/judgments.md): a comment is a line whose first character that is not a space or
 * tab is `#`; a blank line holds only spaces and tabs; an assignment has no leading or trailing
 * space; in double quotes a backslash before any character other than `$`, a backtick, `"` or
 * another backslash stays a literal backslash, as in Bash; a name is a Bash identifier; a repeated
 * name takes its last value.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/env-file/powershell_reader.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/env-file/powershell-reader');

/** The message `claude --debug` logs, as an issue's text. */
export const NOT_PLAIN_ASSIGNMENTS: string = call('NOT_PLAIN_ASSIGNMENTS');

export interface PowershellVariables {
  /** What PowerShell commands receive: nothing when a line is not plain. */
  readonly variables: Readonly<Record<string, string>>;
  readonly issues: readonly PluginIssue[];
}

export interface PowershellReadOptions {
  /** A Windows host drops a variable whose value holds `/` or `\`. */
  readonly windows: boolean;
}

/** The variables a PowerShell command receives out of the text of every file the session's hooks wrote. */
export const readPowershellVariables = (
  text: string,
  options: PowershellReadOptions,
): PowershellVariables => call('readPowershellVariables', text, options);
