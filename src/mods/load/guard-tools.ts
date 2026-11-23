import { ENGINE_ORIGIN } from '../../protocol/mods-events.ts';
import { isRecord } from '../../protocol/is-record.ts';

/**
 * The guard's hold on the tools of a managed MCP server that has connected (spec §14.9.2 item 3):
 * where it loads, a user's mod cannot rewrite "the tool list and tool descriptions of a managed MCP
 * server that has connected". The tool descriptions are kept by not admitting a user's
 * `tool.describe` hook for such a tool (`isManagedTool`); the list is kept here.
 */

/** One tool of a managed MCP server, as the host lists it in the client's pool (`ToolInfo`). */
export interface ManagedToolInfo {
  readonly name: string;
  readonly [field: string]: unknown;
}

const nameOf = (tool: unknown): unknown => (isRecord(tool) ? tool.name : undefined);

/**
 * The pool the client's own `tool.list` resolves with the managed tools as the host gave them: an
 * entry that names one is replaced by the host's copy where it stands, and one the hooks after the
 * guard left out is added at the end. Any answer but `{ value: [...] }` (a `{ deny }`, a malformed
 * value) becomes the managed tools alone: a user's mod cannot empty the pool of them either.
 */
export function keepManagedTools(result: unknown, managed: readonly ManagedToolInfo[]): unknown {
  if (managed.length === 0) return result;
  const byName = new Map(managed.map((tool) => [tool.name, tool]));
  const list = isRecord(result) && Array.isArray(result.value) ? result.value : [];
  const kept = list.map((tool: unknown) => {
    const name = nameOf(tool);
    return typeof name === 'string' ? (byName.get(name) ?? tool) : tool;
  });
  const present = new Set(kept.map(nameOf));
  return { value: [...kept, ...managed.filter((tool) => !present.has(tool.name))] };
}

/** A pool shaped by the client itself: a mod's `$.tool.list` lists mods' tools, never a server's. */
export const isClientsOwn = (origin: { readonly plugin: string }): boolean =>
  origin.plugin === ENGINE_ORIGIN.plugin;

/** Whether a `tool.offer` input names one of the managed tools. */
export function offersManagedTool(input: unknown, managed: readonly ManagedToolInfo[]): boolean {
  const tool = isRecord(input) ? input.tool : undefined;
  return typeof tool === 'string' && managed.some((one) => one.name === tool);
}
