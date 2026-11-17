import { isPluginVariable, type PluginVariable } from './variables.ts';
import type { Substituter } from './substituter.ts';

/**
 * errors.md, "Hook needs quotes around a path with a space" (spec §17.18.8): an `asyncRewake` hook
 * in shell form whose `command` has an interpreter followed by an unquoted path placeholder that
 * resolves to a path with a space does not run, and the conversation view gets
 * `<Event> hook needs quotes around a path with a space (<script>)`.
 *
 * Judgment (spec/judgments.md): the docs name `python3` as an example interpreter; the kit takes the
 * script interpreters below (a `python` with a version suffix included), and the placeholder as
 * `${NAME}` or `$NAME` of the three path variables at the start of the word after the interpreter.
 */
const INTERPRETER = /^(?:python[\d.]*|node|bash|sh|zsh|ruby|perl|bun|deno|php)$/;
const PLACEHOLDER = /^\$(?:\{(\w+)\}|(\w+))/;

/** The notification, `Stop hook needs quotes around a path with a space (check.py)`. */
export const hookNeedsQuotes = (event: string | undefined, script: string): string =>
  `${event === undefined ? 'Hook' : `${event} hook`} needs quotes around a path with a space (${script})`;

/** The last segment of a path, `/` or `\` separated. */
const lastSegment = (path: string): string =>
  path.split(/[\\/]/).findLast((part) => part !== '') ?? path;

/** The path variable an interpreter's first argument starts with, unquoted, and the rest of the word. */
function placeholderAfterInterpreter(
  command: string,
): { readonly name: PluginVariable; readonly rest: string } | undefined {
  const [program, word] = command.trim().split(/\s+/);
  if (program === undefined || word === undefined || !INTERPRETER.test(program)) return undefined;
  const found = PLACEHOLDER.exec(word);
  const name = found?.[1] ?? found?.[2];
  if (found === null || name === undefined || !isPluginVariable(name)) return undefined;
  return { name, rest: word.slice(found[0].length) };
}

/**
 * The script a command would run from an unquoted path that has a space once resolved, or
 * `undefined` when the command is safe as written.
 */
export async function unquotedSpacePath(
  command: string,
  substitution: Substituter,
): Promise<string | undefined> {
  const found = placeholderAfterInterpreter(command);
  if (found === undefined) return undefined;
  const value = await substitution.variable(found.name);
  return /\s/.test(value) ? lastSegment(`${value}${found.rest}`) : undefined;
}
