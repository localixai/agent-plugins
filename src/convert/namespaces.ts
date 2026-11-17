import { listSorted } from '../format/listing.ts';
import { readJson } from '../format/read-json.ts';
import type { PluginFiles } from '../ports/index.ts';
import { isRecord } from '../protocol/is-record.ts';

/**
 * Every client extension namespace a core package carries (core §8), so a conversion loads the
 * package with all of them and reports each one instead of the loader dropping the namespaces the
 * host does not implement unseen (Agent Plugins Extended §18.4.1 item 1): the `extensions` keys of
 * `plugin.json`, of each `mcp.json` server and of each `hooks.json` handler, and every top-level
 * directory named as a reverse-domain namespace. Judgment (spec/judgments.md): a directory counts
 * when its name is two or more dot-separated labels of letters, digits and hyphens, the form core §8
 * gives namespaces (`com.example.client`); `assets/` never does.
 */

const NAMESPACE = /^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$/;

const keysOf = (value: unknown): string[] =>
  isRecord(value) && isRecord(value.extensions) ? Object.keys(value.extensions) : [];

const valuesOf = (value: unknown): unknown[] => (isRecord(value) ? Object.values(value) : []);

async function documentOf(files: PluginFiles, path: string, signal?: AbortSignal) {
  const read = await readJson(files, path, signal);
  return 'value' in read ? read.value : undefined;
}

export async function extensionNamespaces(
  files: PluginFiles,
  signal?: AbortSignal,
): Promise<readonly string[]> {
  const [manifest, mcp, hooks, entries] = await Promise.all([
    documentOf(files, 'plugin.json', signal),
    documentOf(files, 'mcp.json', signal),
    documentOf(files, 'hooks.json', signal),
    listSorted(files, '.', signal),
  ]);
  const servers = isRecord(mcp) ? valuesOf(mcp.mcpServers) : [];
  const handlers = isRecord(hooks)
    ? valuesOf(hooks.hooks).flatMap((list) => (Array.isArray(list) ? list : []))
    : [];
  const directories = (entries ?? [])
    .filter((entry) => entry.isDirectory && NAMESPACE.test(entry.name))
    .map((entry) => entry.name);
  return [
    ...new Set([
      ...keysOf(manifest),
      ...servers.flatMap(keysOf),
      ...handlers.flatMap(keysOf),
      ...directories,
    ]),
  ].toSorted();
}

/** Whether `hooks/hooks.json` declares `modules`: the plugin is a mod (plugins/mods/reference.md, "Files"). */
export async function declaresModules(files: PluginFiles, signal?: AbortSignal): Promise<boolean> {
  const value = await documentOf(files, 'hooks/hooks.json', signal);
  return isRecord(value) && Object.hasOwn(value, 'modules');
}
