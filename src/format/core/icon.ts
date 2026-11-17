import type { DirectoryEntry } from '../../ports/files.ts';

/**
 * Plugin icon candidates (core §4.3.1): a regular file directly in the root `assets/` directory whose
 * whole name is lowercase, `icon` and one image-format extension the client recognises. PNG is the
 * one format every displaying client supports, so it is always recognised. The kit only finds the
 * candidates: reading, decoding and rendering them, and making SVG inert, are the host's, and an
 * unusable candidate never affects the package (core §4.3.1).
 */

const CANDIDATE = /^icon\.([a-z0-9]+)$/;

/**
 * The candidates among the entries of `assets/`, as root-relative paths: PNG first, then the other
 * recognised formats in the order `formats` gives them (judgment: a client may pick any).
 */
export function iconCandidates(
  entries: readonly DirectoryEntry[],
  formats: readonly string[] = [],
): readonly string[] {
  const order = [
    'png',
    ...formats.map((format) => format.toLowerCase()).filter((one) => one !== 'png'),
  ];
  const found = entries.flatMap((entry) => {
    const extension = entry.isFile ? CANDIDATE.exec(entry.name)?.[1] : undefined;
    const rank = extension === undefined ? -1 : order.indexOf(extension);
    return rank < 0 ? [] : [{ rank, path: `assets/${entry.name}` }];
  });
  return found.toSorted((first, second) => first.rank - second.rank).map((one) => one.path);
}
