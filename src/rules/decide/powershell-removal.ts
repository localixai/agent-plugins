import { POWERSHELL_DIALECT } from '../bash/dialect.ts';
import { cmdletCall } from '../bash/powershell.ts';
import { scanCommand } from '../bash/split.ts';
import type { ShellCommand, ShellWord } from '../bash/words.ts';
import { absolutePath } from '../path/posix.ts';
import type { PathContext } from '../types.ts';
import { isSystemDirectory } from './critical-removal.ts';

/**
 * permission-modes.md, "Remove-Item in PowerShell": the two cases Claude Code "denies ... in every
 * mode, without asking you" (`bypassPermissions` included: "The Remove-Item in PowerShell denies
 * also apply in this mode"):
 * - `Remove-Item` on a system path ("the filesystem root and its top-level directories, drive roots
 *   and their top-level directories, and your home directory"), or on a wildcard ("a bare `*`, or
 *   any target ending in `/*` or `\*`, including a glob under a shell variable such as `$dir/*`");
 * - `rd`, `rmdir`, `del` and `erase` run through `cmd` on a system path, where "a PowerShell
 *   variable that follows literal text" counts as empty and "a trailing wildcard counts as the folder
 *   it empties". `CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY=1` turns this case off
 *   (`DecideContext.powershellCmdRemovalDenyOff`, the host's reading of the launch environment).
 */

/** The named parameters of `Remove-Item` whose value is a target. */
const PATH_PARAMETERS: ReadonlySet<string> = new Set(['-path', '-literalpath', '-lp', '-pspath']);
/** Named parameters that take a value that is not a target. */
const VALUE_PARAMETERS: ReadonlySet<string> = new Set([
  '-filter',
  '-include',
  '-exclude',
  '-credential',
  '-stream',
]);

/** The words a `Remove-Item` call removes: its positional arguments and its path parameters' values. */
function removeItemTargets(args: readonly ShellWord[]): readonly ShellWord[] {
  const targets: ShellWord[] = [];
  for (let at = 0; at < args.length; at += 1) {
    const word = args[at];
    if (word === undefined) continue;
    const name = word.value.toLowerCase();
    if (!name.startsWith('-')) targets.push(word);
    else if (PATH_PARAMETERS.has(name) || VALUE_PARAMETERS.has(name)) {
      const value = args[at + 1];
      if (value !== undefined && PATH_PARAMETERS.has(name)) targets.push(value);
      at += 1;
    }
  }
  return targets;
}

const isWildcard = (value: string): boolean => value === '*' || /[/\\]\*$/.test(value);

const systemPath = (text: string, paths: PathContext): boolean =>
  text !== '' && isSystemDirectory(absolutePath(text, paths), paths);

/** A `Remove-Item` target the docs deny: a wildcard, or a literal system path. */
const removeItemDenied = (word: ShellWord, paths: PathContext): boolean =>
  isWildcard(word.value) || (!word.expands && !word.globs && systemPath(word.value, paths));

const CMD_NAMES: ReadonlySet<string> = new Set(['cmd', 'cmd.exe']);
const CMD_RUN_SWITCHES: ReadonlySet<string> = new Set(['/c', '/k']);
const CMD_REMOVALS: ReadonlySet<string> = new Set(['rd', 'rmdir', 'del', 'erase']);

/** What `cmd` runs after `/c` or `/k`, one word per token (a quoted command is split again). */
function cmdTokens(args: readonly ShellWord[]): readonly string[] {
  const run = args.findIndex((word) => CMD_RUN_SWITCHES.has(word.value.toLowerCase()));
  if (run < 0) return [];
  return args.slice(run + 1).flatMap((word) => word.value.split(/\s+/).filter(Boolean));
}

/** A `cmd` target as judged: a variable after literal text is empty, a trailing wildcard is its folder. */
const cmdTarget = (token: string): string =>
  token
    .replace(/\$(?:\{[^}]*\}|[\w:]+)/g, '')
    .replace(/[/\\]\*$/, '\\')
    .replace(/^\*$/, '.');

function cmdRemovalDenied(args: readonly ShellWord[], paths: PathContext): boolean {
  const [verb, ...rest] = cmdTokens(args);
  if (verb === undefined || !CMD_REMOVALS.has(verb.toLowerCase())) return false;
  return rest
    .filter((token) => !token.startsWith('/'))
    .some((token) => systemPath(cmdTarget(token), paths));
}

function deniedCommand(command: ShellCommand, paths: PathContext, cmdOff: boolean): boolean {
  const found = cmdletCall(command);
  if (found === undefined) return false;
  const cmdlet = found.cmdlet.toLowerCase();
  if (cmdlet === 'remove-item') {
    return removeItemTargets(found.args).some((word) => removeItemDenied(word, paths));
  }
  return !cmdOff && CMD_NAMES.has(cmdlet) && cmdRemovalDenied(found.args, paths);
}

/** Whether a PowerShell command holds a removal the docs deny in every mode. */
export function powershellRemovalDenied(
  command: string,
  paths: PathContext,
  cmdCheckOff: boolean,
): boolean {
  const { commands } = scanCommand(command, POWERSHELL_DIALECT);
  return commands.some((each) => deniedCommand(each, paths, cmdCheckOff));
}
