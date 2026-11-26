/**
 * `CLAUDE_CODE_SHELL_PREFIX` (env-vars.md, "Variables"; spec §3.6, §7.3.2.3 item 5, §8.2.1): a
 * command that wraps the shell commands the client spawns. Each runs as `<prefix> '<command line>'`,
 * the whole line one shell-quoted argument the wrapper re-runs with a shell (`exec bash -c "$1"`).
 * Pure: the host runs the `CommandSpec` through its `CommandRunner`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/shell_prefix.rs`,
 * docs/RUST.md).
 */
import type { McpStdioServerConfig } from '../../protocol/official.ts';
import type { CommandSpec, Environment, ShellKind } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/resolve/shell-prefix');

/** The variable's name. */
export const SHELL_PREFIX_VARIABLE: 'CLAUDE_CODE_SHELL_PREFIX' = call('SHELL_PREFIX_VARIABLE');

/** The prefix in force: the variable's value as written, absent when it is unset or blank (as for hooks). */
export const shellPrefixOf = (environment: Pick<Environment, 'get'>): string | undefined =>
  call('shellPrefixOf', environment);

/** One word quoted for a POSIX shell: single quotes, an embedded `'` written `'\''`. */
export const posixQuote = (text: string): string => call('posixQuote', text);

/** `<prefix> '<line>'`: the prefix as written, then the line as its one quoted argument. */
export const shellPrefixed = (prefix: string, line: string): string =>
  call('shellPrefixed', prefix, line);

/**
 * How a stdio MCP server starts (spec §8.2.1, J8.26): without a prefix, its `command` and `args` as
 * an exec; with one, the line of `command` and each `args` element, each POSIX-quoted and joined by
 * spaces, passed quoted once more after the prefix, through bash, the shell hook commands use by
 * default (`shell` when the host's default differs, as on Windows without Git Bash).
 */
export const mcpStdioStart = (
  server: Pick<McpStdioServerConfig, 'command' | 'args'>,
  environment: Pick<Environment, 'get'>,
  shell: ShellKind = 'bash',
): CommandSpec => call('mcpStdioStart', server, environment, shell);
