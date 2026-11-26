/**
 * Builds the Rust core for the npm package (docs/RUST.md): `crates/agent-plugins-wasm` compiled
 * to WebAssembly, its custom sections dropped (debugging aids only), written as base64 into
 * `src/wasm/module.ts`, which `src/wasm/bridge.ts` compiles when the package loads. The file is
 * not committed: run this before the tests, the type check or a build.
 *
 *   bun scripts/wasm/build.ts           build and write src/wasm/module.ts
 *   bun scripts/wasm/build.ts --no-cargo  write it from the module cargo built last
 */
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dir, '..', '..');
const target = 'wasm32-unknown-unknown';

const built = process.argv.includes('--no-cargo')
  ? { exitCode: 0 }
  : Bun.spawnSync(
      [
        'cargo',
        'build',
        '--locked',
        '--profile',
        'wasm',
        '--package',
        'agent-plugins-wasm',
        '--target',
        target,
      ],
      { cwd: root, stdout: 'inherit', stderr: 'inherit' },
    );
if (built.exitCode !== 0) process.exit(built.exitCode ?? 1);

const wasm = new Uint8Array(
  await Bun.file(join(root, 'target', target, 'wasm', 'agent_plugins_wasm.wasm')).arrayBuffer(),
);

/** The module's bytes without its custom sections (id 0: `name`, `producers`, …). */
function stripCustomSections(module: Uint8Array): Uint8Array {
  const kept: Uint8Array[] = [module.subarray(0, 8)];
  let at = 8;
  while (at < module.length) {
    const start = at;
    const id = module[at++] ?? 0;
    let size = 0;
    for (let shift = 0; ; shift += 7) {
      const byte = module[at++];
      if (byte === undefined || shift > 28) throw new Error('a section size past the module');
      size += (byte & 0x7f) * 2 ** shift;
      if ((byte & 0x80) === 0) break;
    }
    const end = at + size;
    if (end > module.length) throw new Error('a section past the module');
    if (id !== 0) kept.push(module.subarray(start, end));
    at = end;
  }
  const out = new Uint8Array(kept.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of kept) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}

const stripped = stripCustomSections(wasm);
const text = Buffer.from(stripped).toString('base64');
writeFileSync(
  join(root, 'src', 'wasm', 'module.ts'),
  `// Written by scripts/wasm/build.ts from crates/agent-plugins-wasm; not committed.\n/** The Rust core as WebAssembly, base64. */\nexport const MODULE = '${text}';\n`,
);
console.log(`src/wasm/module.ts: ${stripped.length} bytes of WebAssembly (${wasm.length} built)`);
