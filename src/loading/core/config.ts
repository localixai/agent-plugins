import { relativeCommandPath } from '../../format/core/command.ts';
import {
  CORE_HOOK_EVENTS,
  readCoreHooks,
  type CoreHookEvent,
  type CoreHookHandler,
  type CoreHooks,
} from '../../format/core/hooks.ts';
import { readCoreMcp, type CoreMcpOptions } from '../../format/core/mcp.ts';
import type { CoreMcpServer } from '../../format/core/mcp-server.ts';
import { resolveCwd, SYMBOLIC_PATHS } from '../../format/core/placeholders.ts';
import type { CoreVersion } from '../../format/core/versions.ts';
import { error, type PluginIssue } from '../../format/issues.ts';
import { locate, type LocateContext } from '../../format/locate.ts';
import { fixedJson } from './locations.ts';

/**
 * `mcp.json` and `hooks.json` of a core package, read from their fixed locations and then held to
 * the filesystem: a `./` command must resolve to a regular file inside the plugin root, and a `cwd`
 * rooted in the plugin root must stay inside it through any link (core §4.1, boundaries 4 and 5).
 * A server or handler that fails is skipped and reported; the rest loads.
 *
 * Judgment (spec/judgments.md): a `./` command that names no file fails command resolution, so the
 * entry is invalid at load (core §7.3.2 item 4), not at launch; a `cwd` that does not exist yet is
 * no load problem, since only its containment is a configuration rule.
 */

/** Why a `./` command does not resolve inside the root, or `undefined` (a bare name resolves at launch). */
async function commandIssue(
  context: LocateContext,
  where: string,
  command: string,
): Promise<string | undefined> {
  const relative = relativeCommandPath(command);
  if (relative === undefined) return undefined;
  const found = await locate(context, where, relative);
  if (found.issue) return found.issue.message;
  return found.stat?.isFile === true
    ? undefined
    : `"command" ${command} names no file in the package`;
}

async function cwdIssue(
  context: LocateContext,
  where: string,
  cwd: string | undefined,
): Promise<string | undefined> {
  if (cwd === undefined) return undefined;
  const resolved = resolveCwd(cwd, SYMBOLIC_PATHS);
  if ('problem' in resolved || resolved.cwd.base !== 'PLUGIN_ROOT') return undefined;
  const found = await locate(context, where, resolved.cwd.below);
  return found.issue?.message;
}

async function serverIssue(
  context: LocateContext,
  name: string,
  server: CoreMcpServer,
): Promise<string | undefined> {
  if (!('command' in server)) return undefined;
  const where = `mcpServers.${name}`;
  return (
    (await commandIssue(context, where, server.command)) ??
    (await cwdIssue(context, where, server.cwd))
  );
}

export interface CoreMcpRead {
  readonly servers: Readonly<Record<string, CoreMcpServer>>;
  readonly issues: readonly PluginIssue[];
}

/** The servers of `mcp.json` that are valid and contained. */
export async function readCoreMcpFile(
  context: LocateContext,
  version: CoreVersion,
  options: CoreMcpOptions,
): Promise<CoreMcpRead> {
  const read = await fixedJson(context, 'mcp.json', 'MCP is disabled');
  if ('absent' in read) return { servers: {}, issues: [] };
  if ('issue' in read) return { servers: {}, issues: [read.issue] };
  const parsed = readCoreMcp(read.value, version, options);
  const checked = await Promise.all(
    Object.entries(parsed.servers).map(async ([name, server]) => ({
      name,
      server,
      problem: await serverIssue(context, name, server),
    })),
  );
  const servers = Object.fromEntries(
    checked.filter((one) => one.problem === undefined).map((one) => [one.name, one.server]),
  );
  const skipped = checked.flatMap((one) =>
    one.problem === undefined
      ? []
      : [error(`mcpServers.${one.name}`, `Server "${one.name}" is skipped: ${one.problem}`)],
  );
  return { servers, issues: [...parsed.issues, ...skipped] };
}

export interface CoreHooksRead {
  readonly hooks: CoreHooks;
  readonly issues: readonly PluginIssue[];
}

/** The handlers of `hooks.json` that are valid and whose `./` command resolves inside the root. */
export async function readCoreHooksFile(
  context: LocateContext,
  version: CoreVersion,
  implemented: readonly string[],
): Promise<CoreHooksRead> {
  const read = await fixedJson(context, 'hooks.json', 'Hooks are disabled');
  if ('absent' in read) return { hooks: {}, issues: [] };
  if ('issue' in read) return { hooks: {}, issues: [read.issue] };
  const parsed = readCoreHooks(read.value, version, implemented);
  const issues: PluginIssue[] = [...parsed.issues];
  const hooks: { [E in CoreHookEvent]?: readonly CoreHookHandler[] } = {};
  for (const event of CORE_HOOK_EVENTS) {
    const handlers = parsed.hooks[event] ?? [];
    const positions = parsed.positions[event] ?? [];
    const kept: CoreHookHandler[] = [];
    for (const [index, handler] of handlers.entries()) {
      // The handler's index in the file, not among the handlers kept so far (core §7.3.2 item 4).
      const where = `hooks.${event}[${positions[index] ?? index}]`;
      const problem = await commandIssue(context, where, handler.command);
      if (problem === undefined) kept.push(handler);
      else issues.push(error(where, `Handler is skipped: ${problem}`));
    }
    if (kept.length > 0) hooks[event] = kept;
  }
  return { hooks, issues };
}
