/**
 * The host's loop over the Rust core (docs/RUST.md, "The boundary", effects and handles): a port,
 * a callback and a signal cross as numbers and stay the host's own objects; needs asked together
 * run together; what the host throws, the call throws, unless the core catches it (and then a
 * rethrow is the very value); a handle's methods call the core.
 */
import { describe, expect, test } from 'bun:test';
import { configDirectoryOf } from '../../src/format/config-directory.ts';
import { listSorted } from '../../src/format/listing.ts';
import { readTextOf } from '../../src/foundation/folder-read.ts';
import { readJson, readJsonWithoutBom } from '../../src/format/read-json.ts';
import type { DirectoryEntry, PluginFiles } from '../../src/ports/files.ts';
import type { ScanFolder } from '../../src/ports/folder-scan.ts';
import { coreModule } from '../../src/wasm/bridge.ts';

const probe = coreModule('registry/probe');

const never = (): Promise<never> => Promise.reject(new Error('not read in this test'));

/** A `PluginFiles` whose `readText` is `readText`; the rest is never called. */
const filesReading = (readText: PluginFiles['readText']): PluginFiles => ({
  readText,
  readBytes: never,
  list: never,
  stat: never,
  exists: () => Promise.resolve(false),
});

const entry = (name: string): DirectoryEntry => ({ name, isFile: true, isDirectory: false });

const join = (base: string, relativePath: string): string => `${base}/${relativePath}`;

describe('a port', () => {
  test('is the host object itself, called with the arguments and signal it was given', async () => {
    const signal = new AbortController().signal;
    const seen: unknown[] = [];
    const files = filesReading((path, given) => {
      seen.push(path, given);
      return Promise.resolve('{"a": 1}');
    });
    expect(await readJson(files, 'plugin.json', signal)).toEqual({ value: { a: 1 } });
    expect(seen).toEqual(['plugin.json', signal]);
    expect(seen[1]).toBe(signal);
  });

  test('that throws makes the call throw the same value', async () => {
    const failure = new Error('EACCES: permission denied');
    const files = filesReading(() => Promise.reject(failure));
    let caught: unknown;
    await readJson(files, 'plugin.json').catch((error: unknown) => {
      caught = error;
    });
    expect(caught).toBe(failure);
  });

  test('that throws at once, before a promise, is the same', async () => {
    const files = filesReading(() => {
      throw new RangeError('no such folder');
    });
    const caught = await readJson(files, 'x.json').catch((error: unknown) => error);
    expect(caught).toBeInstanceOf(RangeError);
  });

  test('answers bytes and lists through the core', async () => {
    const bom = new Uint8Array([0xef, 0xbb, 0xbf, 0x7b, 0x7d]);
    const bytes: PluginFiles = { ...filesReading(never), readBytes: () => Promise.resolve(bom) };
    expect(await readJsonWithoutBom(bytes, 'hooks/hooks.json')).toHaveProperty('problem');
    const listed = { list: () => Promise.resolve([entry('b'), entry('B'), entry('a')]) };
    expect((await listSorted(listed, '.'))?.map(({ name }) => name)).toEqual(['B', 'a', 'b']);
    expect(await listSorted({ list: () => Promise.resolve(undefined) }, '.')).toBeUndefined();
  });
});

test('two needs asked together run together and may answer in any order', async () => {
  const started: string[] = [];
  const answers = new Map<string, (text: string) => void>();
  const files = filesReading(
    (path) =>
      new Promise((resolve) => {
        started.push(path);
        answers.set(path, resolve);
      }),
  );
  const both = probe.async<readonly string[]>('both', files, 'one', 'two');
  await Promise.resolve();
  expect(started).toEqual(['one', 'two']);
  answers.get('two')?.('2');
  answers.get('one')?.('1');
  expect(await both).toEqual(['1', '2']);
});

describe('a port with both kinds of method', () => {
  test('is answered at once where it returns a value, awaited where it returns a promise', async () => {
    const asked: string[] = [];
    const locations = {
      homeDir: (): string => {
        asked.push('homeDir');
        return '/home/u';
      },
      pluginData: (plugin: string): Promise<string> => {
        asked.push(`pluginData ${plugin}`);
        return Promise.resolve(`/home/u/.claude/plugins/data/${plugin}`);
      },
    };
    expect(await probe.async('located', locations, 'lint')).toEqual([
      '/home/u',
      '/home/u/.claude/plugins/data/lint',
    ]);
    expect(asked).toEqual(['homeDir', 'pluginData lint']);
  });

  test('hands over a port of its own, whose property is read as a need', async () => {
    const folder = { dir: '/p/.claude', hostPath: (path: string): string => `/p/.claude/${path}` };
    const scan = {
      open: (dir: string) => Promise.resolve(dir === '/p/.claude' ? folder : undefined),
    };
    expect(await probe.async('opened', scan, '/p/.claude')).toBe('/p/.claude');
    expect(await probe.async('opened', scan, '/q')).toBeUndefined();
  });
});

/** A `ScanFolder` at `/p/.claude` whose files' `readText` is `readText`. */
const folderReading = (readText: PluginFiles['readText']): ScanFolder => ({
  dir: '/p/.claude',
  files: filesReading(readText),
  hostPath: (path) => `/p/.claude/${path}`,
  canonicalPath: () => Promise.resolve(undefined),
});

const refusing = (): never => {
  throw new RangeError('no');
};

describe('a throw the core catches', () => {
  test('is what the catch makes of it, with the message and the name of what was thrown', async () => {
    const folder = folderReading(() => Promise.reject(new RangeError('file is 2 MiB')));
    expect(await readTextOf(folder, 'big/SKILL.md')).toEqual({
      text: undefined,
      issue: {
        severity: 'warning',
        path: '/p/.claude/big/SKILL.md',
        message: 'cannot be read: file is 2 MiB; skipped',
      },
    });
    expect(await probe.async('caught', refusing)).toEqual(['no', 'RangeError']);
    // A host that rejects with a value that is no Error: its text, and no name.
    // oxlint-disable-next-line prefer-promise-reject-errors -- the host under test throws a string.
    expect(await probe.async('caught', () => Promise.reject('plain'))).toEqual(['plain', null]);
  });

  test('that the catch throws again is the very value the host threw', async () => {
    const failure = new Error('aborted');
    const abort = new AbortController();
    abort.abort();
    const folder = folderReading(() => Promise.reject(failure));
    const caught = await readTextOf(folder, 'a.md', abort.signal).catch((error: unknown) => error);
    expect(caught).toBe(failure);
  });
});

describe('a callback', () => {
  test('is called as the core asks, in a call that answers at once', () => {
    expect(configDirectoryOf('/home/u', undefined, join)).toBe('/home/u/.claude');
    expect(configDirectoryOf('/home/u', '/etc/claude', join)).toBe('/etc/claude');
  });

  test('that throws makes the call throw the same value', () => {
    const failure = new TypeError('bad base');
    expect(() =>
      configDirectoryOf('/home/u', undefined, () => {
        throw failure;
      }),
    ).toThrow(failure);
  });

  test('that returns a promise to a synchronous call is refused', () => {
    // A `Join` that breaks its type: the wrapper calls it synchronously.
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the broken host under test.
    const later = (() => Promise.resolve('x')) as unknown as typeof join;
    expect(() => configDirectoryOf('/home/u', undefined, later)).toThrow(/promise/u);
  });
});

test('a handle is an object whose methods call the core, sync and async', async () => {
  interface Counter {
    add(by: number): number;
    addLater(by: () => Promise<number>): Promise<number>;
  }
  const counter = probe<Counter>('counter', 1);
  expect(Object.keys(counter)).toEqual(['add', 'addLater']);
  expect(counter.add(2)).toBe(3);
  expect(await counter.addLater(() => Promise.resolve(4))).toBe(7);
  const other = probe<Counter>('counter', 10);
  expect(other.add(1)).toBe(11);
  expect(counter.add(0)).toBe(7);
});

test('a handle given back is the core object it was, not a copy or a port', () => {
  interface Counter {
    add(by: number): number;
  }
  const counter = probe<Counter>('counter', 1);
  expect(probe<number>('addTo', counter, 2)).toBe(3);
  expect(counter.add(0)).toBe(3);
  expect(() => probe('addTo', { add: () => 0 }, 2)).toThrow(/Own|handle|decode|port/u);
});
