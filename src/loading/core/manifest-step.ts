import { readCoreManifest, type CoreManifestResult } from '../../format/core/manifest.ts';
import { error } from '../../format/issues.ts';
import type { LocateContext } from '../../format/locate.ts';
import { fixedJson } from './locations.ts';

/**
 * The first step of a core load (core §5.1: the manifest is loaded and validated before any
 * component is discovered): a root `plugin.json` that is missing, not a regular file, outside the
 * plugin root (core §4.1, boundary 1), not JSON or invalid by core §5 rejects the package.
 */

const FILE = 'plugin.json';

export async function coreManifestStep(
  context: LocateContext,
  implemented: readonly string[],
): Promise<CoreManifestResult> {
  const read = await fixedJson(context, FILE, 'The package is rejected');
  if ('absent' in read) {
    return {
      manifest: undefined,
      version: undefined,
      issues: [error(FILE, 'A core package needs a plugin.json at its root (core §4.1)')],
    };
  }
  if ('issue' in read) {
    return { manifest: undefined, version: undefined, issues: [{ ...read.issue, path: FILE }] };
  }
  return readCoreManifest(read.value, implemented);
}
