import { warning, type PluginIssue } from './issues.ts';

/**
 * An `mcpServers` or `lspServers` file path that is absolute or has a `..` part is not read, even
 * when it stays inside the plugin; the rest of the plugin loads, without the servers that file
 * defines (spec §2.5 item 2, §17.18.12). The validator still reports such a path as the error §2.5
 * gives; loading drops it with this message instead.
 */

const COMPONENT = { mcpServers: 'mcp-servers', lspServers: 'lsp-servers' } as const;
type ServerKey = keyof typeof COMPONENT;
const SERVER_KEYS: readonly ServerKey[] = ['mcpServers', 'lspServers'];

const isAbsolute = (path: string): boolean => /^(?:[/\\]|[A-Za-z]:)/.test(path);
const hasParentPart = (path: string): boolean => path.split(/[/\\]/).includes('..');

/** A server file path loading does not read. */
const notRead = (value: unknown): value is string =>
  typeof value === 'string' && (isAbsolute(value) || hasParentPart(value));

/** `mcp-servers path not read: ./sub/../x.json is absolute or has a ".." part`. */
const notReadIssue = (key: ServerKey, path: string): PluginIssue =>
  warning(key, `${COMPONENT[key]} path not read: ${path} is absolute or has a ".." part`);

function dropOne(key: ServerKey, value: unknown): { kept: unknown; issues: PluginIssue[] } {
  if (notRead(value)) return { kept: undefined, issues: [notReadIssue(key, value)] };
  if (!Array.isArray(value)) return { kept: value, issues: [] };
  const dropped = value.filter(notRead);
  return {
    kept: value.filter((entry: unknown) => !notRead(entry)),
    issues: dropped.map((path) => notReadIssue(key, path)),
  };
}

/** The manifest without the server paths loading does not read, and a message for each. */
export function withoutUnreadServerPaths(manifest: Readonly<Record<string, unknown>>): {
  readonly manifest: Readonly<Record<string, unknown>>;
  readonly issues: readonly PluginIssue[];
} {
  const kept: Record<string, unknown> = { ...manifest };
  const issues: PluginIssue[] = [];
  for (const key of SERVER_KEYS) {
    if (!Object.hasOwn(manifest, key)) continue;
    const one = dropOne(key, manifest[key]);
    issues.push(...one.issues);
    if (one.kept === undefined) delete kept[key];
    else kept[key] = one.kept;
  }
  return { manifest: kept, issues };
}
