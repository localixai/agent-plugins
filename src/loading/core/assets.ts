import { iconCandidates } from '../../format/core/icon.ts';
import { listSorted } from '../../format/listing.ts';
import { locate, type LocateContext } from '../../format/locate.ts';

/**
 * The package's files that are data, not components: icon candidates under `assets/` (core §4.3.1)
 * and the extension directories of the namespaces the host implements (core §8.2). A candidate or a
 * directory that resolves outside the plugin root is left out, its access denied (core §4.1,
 * boundary 6); neither ever invalidates the package. `assets/` is never an extension directory (§8).
 */

const ASSETS = 'assets';

/** The usable-looking icon candidates, PNG first. */
export async function coreIcons(
  context: LocateContext,
  formats: readonly string[],
): Promise<readonly string[]> {
  const folder = await locate(context, ASSETS, ASSETS, true);
  if (folder.issue || folder.stat?.isDirectory !== true) return [];
  const entries = (await listSorted(context.files, ASSETS, context.signal)) ?? [];
  const candidates = iconCandidates(entries, formats);
  const contained = await Promise.all(
    candidates.map(async (path) => {
      const found = await locate(context, ASSETS, path, true);
      return found.stat?.isFile === true ? [path] : [];
    }),
  );
  return contained.flat();
}

/** The top-level directories named exactly for an implemented namespace, in name order. */
export async function coreExtensionDirectories(
  context: LocateContext,
  namespaces: readonly string[],
): Promise<readonly string[]> {
  const wanted = namespaces.filter((name) => name !== ASSETS);
  if (wanted.length === 0) return [];
  const entries = (await listSorted(context.files, '.', context.signal)) ?? [];
  const found = await Promise.all(
    entries
      .filter((entry) => wanted.includes(entry.name))
      .map(async (entry) => {
        const at = await locate(context, entry.name, entry.name, true);
        return at.stat?.isDirectory === true ? [entry.name] : [];
      }),
  );
  return found.flat();
}
