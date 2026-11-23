import type { FolderScan } from '../ports/folder-scan.ts';
import { looksLikeBareRepository } from '../skills/index.ts';
import type { ExpandOptions } from './session-skill-expand.ts';

/**
 * Whether the closest repository at or above `cwd` looks bare (Agent Plugins Extended §4.8.5 item
 * 3, "Bare repository folders"): the folders of the host's repository chain, nearest first, until
 * one holds `.git` (a working tree: the closest repository, not bare) or looks bare itself (`HEAD`
 * beside `refs` or `commondir`). Read one after another, as the first answer ends the walk.
 */
async function looksBareAt(
  folders: FolderScan,
  cwd: string,
  signal: AbortSignal,
): Promise<boolean> {
  const { directories } = await folders.repository(cwd, signal);
  for (const directory of directories) {
    // Sequential on purpose: the nearest folder that answers decides, the rest are not read.
    const entries = (await (await folders.open(directory, signal))?.files.list('.', signal)) ?? [];
    if (entries.some((entry) => entry.name === '.git')) return false;
    if (looksLikeBareRepository(entries)) return true;
  }
  return false;
}

/**
 * `bareRepository` for an expansion: the host's answer when it gave one, else read through the
 * folder-scan port. Only a body with a `!` can hold an injected command, so no other body reads a
 * folder; without the port nothing is read and the rule does not apply.
 */
export async function bareRepositoryFor(
  request: Pick<ExpandOptions, 'bareRepository' | 'signal'>,
  where: { readonly folders: FolderScan | undefined; readonly cwd: string; readonly body: string },
): Promise<boolean | undefined> {
  if (request.bareRepository !== undefined) return request.bareRepository;
  if (where.folders === undefined || !where.body.includes('!')) return undefined;
  return looksBareAt(where.folders, where.cwd, request.signal);
}
