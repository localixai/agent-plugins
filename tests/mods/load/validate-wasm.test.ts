/**
 * The WebAssembly files a hooks module compiles. Agent Plugins Extended reference.md §10.4.2, the
 * table's last row (2.1.326 on; the messages Claude Code 2.1.330's): a `$.wasm.compile` path is a
 * string literal naming a file, relative to the calling file, that parses as a WebAssembly module;
 * `<absolute path>: no such file` is an error, and a module that breaks a rule does not load;
 * `<path>: WebAssembly.Module doesn't parse at byte <n>: <reason>` is a warning. §10.4.3 and
 * Judgment J10.2: the `wasm <path>:` line with its size and SHA-256, then `imports:` and `exports:`,
 * `nothing` when none (judgments, outside `spec(...)`).
 */
import { createHash } from 'node:crypto';
import { describe, expect, test } from 'bun:test';
import { validateMod } from '../../../src/mods/load/commands/validate.ts';
import { readWasm } from '../../../src/mods/load/commands/wasm-read.ts';
import { sha256Hex } from '../../../src/mods/load/commands/sha256.ts';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { memoryFiles } from '../../support/ports.ts';
import { spec } from '../../support/spec.ts';

type Compiling = { wasm: { compile(source: unknown): Promise<unknown> } };
type Next = (e: unknown) => unknown;

/** The smallest module: the header alone. */
const EMPTY = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
/** A module that imports `env.log` and exports `run`, both functions. */
const LINKED = new Uint8Array(
  [
    [...EMPTY],
    [0x01, 0x04, 0x01, 0x60, 0x00, 0x00],
    [0x02, 0x0b, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x03, 0x6c, 0x6f, 0x67, 0x00, 0x00],
    [0x03, 0x02, 0x01, 0x00],
    [0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x01],
    [0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b],
  ].flat(),
);
const NOT_WASM = new TextEncoder().encode('not a module');

const sha = (bytes: Uint8Array): string => createHash('sha256').update(bytes).digest('hex');

const validate = (calls: string, files: Record<string, string | Uint8Array>) =>
  validateMod({
    files: memoryFiles({
      '.claude-plugin/plugin.json': JSON.stringify({ name: 'probe', version: '0.1.0' }),
      'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
      'hooks/register.js': `export function register(on) {\n  on('session.start', async ($, e, next) => {\n${calls}\n    return next(e)\n  })\n}`,
      ...files,
    }),
    target: '.',
    root: '/work/probe',
  });

describe('§10.4.2: a file $.wasm.compile names', () => {
  spec('mods.create.Check what Claude Code reads from your mod', () => {
    test('one that is not there is an error at the module, named by its absolute path', async () => {
      const report = await validate("    await $.wasm.compile('./gone.wasm')", {});
      expect(report.exitCode).toBe(1);
      expect(report.contents[0]?.errors).toEqual([
        {
          path: 'modules../register.js',
          message:
            'probe: /work/probe/hooks/register.js:3: /work/probe/hooks/gone.wasm: no such file; $ is always spelled $.noun.event(...) at the call site, on is always on("<event>", hook), and next.to always next.to(e, "<tier>")',
        },
      ]);
    });

    test('one that does not parse is a warning: the run passes with warnings', async () => {
      const report = await validate("    await $.wasm.compile('./bad.wasm')", {
        'hooks/bad.wasm': NOT_WASM,
      });
      expect(report.exitCode).toBe(0);
      expect(report.lines.at(-1)).toBe('✔ Validation passed with warnings');
      expect(report.contents[0]?.warnings.map((entry) => entry.message)).toEqual([
        expect.stringContaining(
          "hooks/bad.wasm: WebAssembly.Module doesn't parse at byte 0: module doesn't start with '\\0asm'",
        ),
      ]);
    });

    test('a module naming a file that is not there does not load', async () => {
      const register = registerOf((on) => {
        on('session.start', async ($: Compiling, e: unknown, next: Next) => {
          await $.wasm.compile('./gone.wasm');
          return next(e);
        });
      });
      const s = session({ probe: register }, { firesSessionStart: false });
      const report = await s.host.load([modPlugin(inline('probe'), register)]);
      expect(report.lines.join('\n')).toContain('hooks/gone.wasm: no such file');
      expect(s.host.active()).not.toContain('probe');
    });

    test('bytes, or a path the scan cannot read, name no file', async () => {
      const report = await validate(
        '    await $.wasm.compile(new Uint8Array(8))\n    await $.wasm.compile(e.path)',
        {},
      );
      expect(report.exitCode).toBe(0);
      expect(report.lines.some((line) => line.includes(' wasm '))).toBe(false);
    });
  });
});

describe('§10.4.3 and J10.2: the wasm lines (outside spec(...))', () => {
  test('the size and SHA-256 of each file, then its imports and exports, nothing when none', async () => {
    const report = await validate(
      "    await $.wasm.compile('./a.wasm')\n    await $.wasm.compile('../lib/linked.wasm')",
      { 'hooks/a.wasm': EMPTY, 'lib/linked.wasm': LINKED },
    );
    expect(report.lines).toEqual(
      expect.arrayContaining([
        `  ❯ ./register.js wasm hooks/a.wasm: 8 bytes, sha256 ${sha(EMPTY)}`,
        '  ❯ ./register.js wasm hooks/a.wasm imports: nothing',
        '  ❯ ./register.js wasm hooks/a.wasm exports: nothing',
        `  ❯ ./register.js wasm lib/linked.wasm: ${LINKED.length} bytes, sha256 ${sha(LINKED)}`,
        '  ❯ ./register.js wasm lib/linked.wasm imports: env.log (function)',
        '  ❯ ./register.js wasm lib/linked.wasm exports: run (function)',
      ]),
    );
    expect(report.contents[0]?.wasm).toEqual([
      { path: 'hooks/a.wasm', bytes: 8, sha256: sha(EMPTY), imports: [], exports: [] },
      {
        path: 'lib/linked.wasm',
        bytes: LINKED.length,
        sha256: sha(LINKED),
        imports: ['env.log (function)'],
        exports: ['run (function)'],
      },
    ]);
  });

  test("the imports and exports read are the engine's", () => {
    const module = new WebAssembly.Module(LINKED);
    const read = readWasm(LINKED);
    expect(read).toEqual({
      imports: WebAssembly.Module.imports(module).map(
        (one) => `${one.module}.${one.name} (${one.kind})`,
      ),
      exports: WebAssembly.Module.exports(module).map((one) => `${one.name} (${one.kind})`),
    });
  });

  test("where a module does not parse, the byte and reason are the engine's", () => {
    const cases = [
      new Uint8Array([0, 0x61, 0x73]),
      NOT_WASM,
      new Uint8Array([0, 0x61, 0x73, 0x6d, 2, 0, 0, 0]),
      new Uint8Array([...EMPTY, 0x01, 0x10, 0x01]),
      new Uint8Array([...EMPTY, 0x20, 0x00]),
      new Uint8Array([...EMPTY, 0x02, 0x03, 0x01, 0x03, 0x65]),
    ];
    for (const bytes of cases) {
      const read = readWasm(bytes);
      const ours =
        'at' in read ? `WebAssembly.Module doesn't parse at byte ${read.at}: ${read.reason}` : 'ok';
      expect(() => new WebAssembly.Module(bytes)).toThrow(ours);
    }
  });

  test('the SHA-256 is the standard digest', () => {
    for (const length of [0, 1, 55, 56, 64, 1000]) {
      const bytes = new Uint8Array(length).map((_, at) => (at * 31 + 7) & 0xff);
      expect(sha256Hex(bytes)).toBe(sha(bytes));
    }
  });
});

describe('§17.13.4: the hooks module in its result (outside spec(...): the item is a judgment)', () => {
  test('modules names the module from the plugin root, its hooks and what it uses', async () => {
    const report = await validate("    await $.http.fetch('https://api.example.com/v1')", {});
    expect(report.contents[0]?.modules).toEqual([
      {
        module: 'hooks/register.js',
        hooks: ['session.start'],
        uses: {
          events: ['session.start'],
          calls: ['http.fetch'],
          hosts: { named: ['api.example.com'], unread: [] },
        },
      },
    ]);
  });

  test('a module whose code cannot be read: modules is null', async () => {
    const report = await validateMod({
      files: memoryFiles({
        '.claude-plugin/plugin.json': JSON.stringify({ name: 'probe', version: '0.1.0' }),
        'hooks/hooks.json': JSON.stringify({ modules: ['./register.js'] }),
      }),
      target: '.',
    });
    expect(report.contents.find((file) => file.file === './register.js')?.modules).toBeNull();
  });
});
