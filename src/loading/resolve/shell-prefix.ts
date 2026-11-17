import type { McpStdioServerConfig } from '../../protocol/official.ts';
import type { CommandSpec, Environment, ShellKind } from '../../ports/index.ts';

/**
 * `CLAUDE_CODE_SHELL_PREFIX` (env-vars.md, "Variables"; spec §3.6, §7.3.2.3 item 5, §8.2.1): a
 * command that wraps the shell commands the client spawns. Each runs as `<prefix> '<command line>'`,
 * the whole line one shell-quoted argument the wrapper re-runs with a shell (`exec bash -c "$1"`).
 * Pure: the host runs the `CommandSpec` through its `CommandRunner`.
 */

/** The variable's name. */
export const SHELL_PREFIX_VARIABLE = 'CLAUDE_CODE_SHELL_PREFIX';

/** The prefix in force: the variable's value as written, absent when it is unset or blank (as for hooks). */
export const shellPrefixOf = (environment: Pick<Environment, 'get'>): string | undefined => {
  const value = environment.get(SHELL_PREFIX_VARIABLE);
  return value === undefined || value.trim() === '' ? undefined : value;
};

/** One word quoted for a POSIX shell: single quotes, an embedded `'` written `'\''`. */
export const posixQuote = (text: string): string => `'${text.replaceAll("'", `'\\''`)}'`;

/** `<prefix> '<line>'`: the prefix as written, then the line as its one quoted argument. */
export const shellPrefixed = (prefix: string, line: string): string =>
  `${prefix} ${posixQuote(line)}`;

/**
 * How a stdio MCP server starts (spec §8.2.1, J8.26): without a prefix, its `command` and `args` as
 * an exec; with one, the line of `command` and each `args` element, each POSIX-quoted and joined by
 * spaces, passed quoted once more after the prefix, through bash, the shell hook commands use by
 * default (`shell` when the host's default differs, as on Windows without Git Bash).
 */
export function mcpStdioStart(
  server: Pick<McpStdioServerConfig, 'command' | 'args'>,
  environment: Pick<Environment, 'get'>,
  shell: ShellKind = 'bash',
): CommandSpec {
  const args = server.args ?? [];
  const prefix = shellPrefixOf(environment);
  if (prefix === undefined) return { form: 'exec', command: server.command, args };
  const line = [server.command, ...args].map(posixQuote).join(' ');
  return { form: 'shell', command: shellPrefixed(prefix, line), shell };
}
