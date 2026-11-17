import { error, type PluginIssue } from '../../format/issues.ts';
import { locate, locateContext, type LocateContext } from '../../format/locate.ts';
import { readJson } from '../../format/read-json.ts';
import type { FileStat, PluginFiles } from '../../ports/index.ts';

/**
 * A fixed location of a core package (core §6): absent is no error (§6.2); present but of the wrong
 * filesystem kind, outside the plugin root (§4.1, boundary 2) or not checkable makes that component
 * type invalid while the others load. Containment is `locate`'s, the one owner of it in the kit.
 */

export type FixedLocation =
  | { readonly absent: true }
  | { readonly issue: PluginIssue }
  | { readonly present: true };

export const coreLocate = (files: PluginFiles, signal?: AbortSignal): LocateContext =>
  locateContext(files, signal === undefined ? {} : { signal });

type Kind = 'file' | 'directory';

/** What each expected kind is called and how a stat shows it. */
const KIND: {
  readonly [K in Kind]: { readonly label: string; readonly is: (stat: FileStat) => boolean };
} = {
  file: { label: 'a regular file', is: (stat) => stat.isFile },
  directory: { label: 'a directory', is: (stat) => stat.isDirectory },
};

/** Where a fixed location stands, for a component whose location must be a `file` or a `directory`. */
export async function fixedLocation(
  context: LocateContext,
  path: string,
  kind: Kind,
): Promise<FixedLocation> {
  const found = await locate(context, path, path, true);
  if (found.issue) return { issue: found.issue };
  if (found.stat === undefined) {
    const other = await context.files.stat(path, context.signal);
    return other === undefined ? { absent: true } : { issue: wrongKind(path, kind) };
  }
  return KIND[kind].is(found.stat) ? { present: true } : { issue: wrongKind(path, kind) };
}

const wrongKind = (path: string, kind: Kind): PluginIssue =>
  error(path, `${path} is not ${KIND[kind].label}: the component type is invalid (core §6.2)`);

/** A fixed JSON file's parsed value, or the issue that disables its component type. */
export async function fixedJson(
  context: LocateContext,
  path: string,
  verdict: string,
): Promise<
  { readonly absent: true } | { readonly issue: PluginIssue } | { readonly value: unknown }
> {
  const where = await fixedLocation(context, path, 'file');
  if (!('present' in where)) return where;
  const read = await readJson(context.files, path, context.signal);
  return 'problem' in read
    ? { issue: error(path, `${verdict}: ${path} is not valid JSON: ${read.problem}`) }
    : { value: read.value };
}
