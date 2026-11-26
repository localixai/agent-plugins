/**
 * The `bun test --preload` of a recording run (scripts/rust/record.ts): `src/` is loaded with its
 * exported functions wrapped (transform.ts), and the test files and their support with `bun:test`
 * replaced by the context-keeping copy (bun-test.ts). Nothing else changes.
 */
import { plugin } from 'bun';
import { afterAll } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { flush } from './store.ts';
import { redirectBunTest, wrapExports } from './transform.ts';

const root = join(import.meta.dir, '..', '..', '..');
const source = join(root, 'src');
const recorder = join(import.meta.dir, 'store.ts');
const bunTest = join(import.meta.dir, 'bun-test.ts');
const escaped = (path: string): string => path.replaceAll(/[.*+?^${}()|[\]\\/]/gu, '\\$&');
const inSource = new RegExp(`^${escaped(source)}/.*\\.ts$`, 'u');
const inTests = new RegExp(`^${escaped(join(root, 'tests'))}/.*\\.ts$`, 'u');

plugin({
  name: 'agent-plugins-recorder',
  setup(build) {
    build.onLoad({ filter: inSource }, ({ path }) => {
      const module = relative(source, path).replace(/\.ts$/u, '');
      return { contents: wrapExports(readFileSync(path, 'utf8'), module, recorder), loader: 'ts' };
    });
    build.onLoad({ filter: inTests }, ({ path }) => ({
      contents: redirectBunTest(readFileSync(path, 'utf8'), bunTest),
      loader: 'ts',
    }));
  },
});

afterAll(flush);
