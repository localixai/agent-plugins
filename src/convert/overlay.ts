import type { DirectoryEntry, FileStat, PluginFiles } from '../ports/index.ts';
import type { ConvertedFile } from './types.ts';

/**
 * A plugin folder as it would be once a conversion's files are written (Agent Plugins Extended
 * §18.4.1 item 3): reads of a written path answer with its text, the folders above it exist, and
 * every other read goes to the folder itself. Nothing is written; it lets the target's own loader
 * and validator judge the output before a host writes it.
 */

const encoder = new TextEncoder();

const parentsOf = (path: string): string[] => {
  const parts = path.split('/');
  return parts.slice(0, -1).map((_part, index) => parts.slice(0, index + 1).join('/'));
};

const childOf = (folder: string, path: string): string | undefined => {
  const prefix = folder === '.' ? '' : `${folder}/`;
  if (!path.startsWith(prefix)) return undefined;
  return path.slice(prefix.length).split('/')[0];
};

const fileStat = (text: string): FileStat => ({
  isFile: true,
  isDirectory: false,
  isSymlink: false,
  size: encoder.encode(text).length,
});
const FOLDER: FileStat = { isFile: false, isDirectory: true, isSymlink: false, size: 0 };

export function overlayFiles(base: PluginFiles, written: readonly ConvertedFile[]): PluginFiles {
  const texts = new Map(written.map((file) => [file.path, file.text]));
  const folders = new Set(written.flatMap((file) => parentsOf(file.path)));
  const writtenEntries = (folder: string): DirectoryEntry[] => {
    const names = new Map<string, boolean>();
    for (const path of [...texts.keys(), ...folders]) {
      const name = childOf(folder, path);
      if (name !== undefined && name !== '')
        names.set(name, texts.has(folder === '.' ? name : `${folder}/${name}`));
    }
    return [...names].map(([name, isFile]) => ({ name, isFile, isDirectory: !isFile }));
  };
  return {
    readText: async (path, signal) => texts.get(path) ?? base.readText(path, signal),
    readBytes: async (path, signal) => {
      const text = texts.get(path);
      return text === undefined ? base.readBytes(path, signal) : encoder.encode(text);
    },
    stat: async (path, signal) => {
      const text = texts.get(path);
      if (text !== undefined) return fileStat(text);
      return (await base.stat(path, signal)) ?? (folders.has(path) ? FOLDER : undefined);
    },
    exists: async (path, signal) =>
      texts.has(path) || folders.has(path) || base.exists(path, signal),
    list: async (path, signal) => {
      const listed = await base.list(path, signal);
      const added = writtenEntries(path);
      if (listed === undefined && !folders.has(path) && path !== '.') return undefined;
      const kept = (listed ?? []).filter((entry) => !added.some((one) => one.name === entry.name));
      return [...kept, ...added];
    },
  };
}
