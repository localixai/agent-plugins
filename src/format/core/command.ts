import { escapesAsWritten, rootRelative } from '../path-text.ts';

/**
 * The `command` of a stdio MCP server or a hook handler (core §7.2.1, §7.3.1): one executable token,
 * never a shell command string, and either a bare executable name, resolved by the platform's
 * search, or a plugin-relative path that begins with `./`, resolved against the plugin root and
 * contained in it (core §4.1 item 4). No placeholder is expanded in it (core §9.2).
 *
 * Judgment (spec/judgments.md): a bare name is one token when it holds no whitespace and no path
 * separator, so `npx tsx`, `bin/server`, `/usr/bin/node`, `../server` and `${PLUGIN_ROOT}/server`
 * are refused; a `./` path is refused when it climbs with `..` or holds a backslash as written.
 */

const RELATIVE = './';

/** The root-relative path a `./` command names, or `undefined` for a bare name. */
export const relativeCommandPath = (command: string): string | undefined =>
  command.startsWith(RELATIVE) ? rootRelative(command.slice(RELATIVE.length)) : undefined;

function relativeProblem(command: string): string | undefined {
  const rest = command.slice(RELATIVE.length);
  if (/\s/.test(rest)) return 'must be one executable token, not a shell command';
  if (escapesAsWritten(rest, 'posix'))
    return 'is a plugin-relative path that leaves the plugin root';
  return rootRelative(rest) === '.' ? 'is a plugin-relative path that names no file' : undefined;
}

function bareProblem(command: string): string | undefined {
  if (/\s/.test(command)) return 'must be one executable token, not a shell command';
  if (command.includes('${')) return 'is not expanded: a placeholder cannot appear in "command"';
  if (/[\\/]/.test(command) || command === '.' || command === '..') {
    return 'must be a bare executable name or a path that begins with "./"';
  }
  return undefined;
}

/** Why a `command` value breaks core §7.2.1 / §7.3.1, or `undefined` when it is a valid token. */
export function commandProblem(command: unknown): string | undefined {
  if (typeof command !== 'string' || command.length === 0) return 'must be a non-empty string';
  return command.startsWith(RELATIVE) ? relativeProblem(command) : bareProblem(command);
}
