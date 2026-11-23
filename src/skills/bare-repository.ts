import type { DirectoryEntry } from '../ports/files.ts';

/**
 * Whether a folder looks like a bare Git repository (skills.md, "Permission checks on injected
 * commands", §4.8.5 item 3): it holds a `HEAD` file directly beside `refs` or `commondir`. The host
 * lists the closest repository folder at or above the working directory and passes the answer as
 * `bareRepository` of the expand request.
 */
export function looksLikeBareRepository(entries: readonly DirectoryEntry[]): boolean {
  const head = entries.some((entry) => entry.name === 'HEAD' && entry.isFile);
  return head && entries.some((entry) => entry.name === 'refs' || entry.name === 'commondir');
}
