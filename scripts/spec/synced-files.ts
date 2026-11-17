/**
 * The synced files as they are on disk: the named files of `SYNCED`, and every file under the
 * `SYNCED_FOLDERS`, each by its path below `spec/` with its bytes and their SHA-256. A symbolic link
 * counts as the path it points to, as git and `spec:sync` record it.
 */
import { existsSync } from 'node:fs';
import { lstat, readdir, readlink } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { SPEC_DIR, SYNCED, SYNCED_FOLDERS, hashOf } from './source.ts';

interface OnDisk {
  readonly bytes: Uint8Array;
  readonly hash: string;
}

async function bytesOf(path: string): Promise<Uint8Array> {
  const stat = await lstat(path);
  if (stat.isSymbolicLink()) return new TextEncoder().encode(await readlink(path));
  return Bun.file(path).bytes();
}

/** Every file and link under a folder, without following a link into the folder it names. */
async function underFolder(folder: string): Promise<string[]> {
  if (!existsSync(folder)) return [];
  const found: string[] = [];
  for (const entry of await readdir(folder, { withFileTypes: true })) {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) found.push(...(await underFolder(path)));
    else found.push(path);
  }
  return found;
}

export async function syncedOnDisk(): Promise<Map<string, OnDisk>> {
  const paths = [
    ...SYNCED.map((name) => join(SPEC_DIR, name)).filter((path) => existsSync(path)),
    ...(await Promise.all(SYNCED_FOLDERS.map((name) => underFolder(join(SPEC_DIR, name))))).flat(),
  ];
  const found = new Map<string, OnDisk>();
  for (const path of paths) {
    const bytes = await bytesOf(path);
    found.set(relative(SPEC_DIR, path), { bytes, hash: hashOf(bytes) });
  }
  return found;
}
