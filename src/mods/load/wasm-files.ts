import type { PluginFiles } from '../../ports/index.ts';
import { sha256Hex } from './commands/sha256.ts';
import { readWasm } from './commands/wasm-read.ts';
import type { ScannedWasm } from './scan/index.ts';

/**
 * The WebAssembly files a hooks module's `$.wasm.compile` calls name, read as the static analysis
 * reads them (Agent Plugins Extended §10.4.2, the last row of the table; Claude Code 2.1.330): a file
 * that is not there is an error, and the module does not load; one that does not parse is a warning.
 * What the validator lists of each (§10.4.3, `wasm <path>:`) comes from the same read.
 */

/** One file read: its size, its SHA-256 and, when it parses, its imports and exports. */
export interface WasmFacts {
  /** As the scan names it: from the plugin root. */
  readonly path: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly imports?: readonly string[];
  readonly exports?: readonly string[];
}

/** A rule a file breaks: `message` is the table's text, `line` where the module first names it. */
export interface WasmProblem {
  readonly severity: 'error' | 'warning';
  readonly message: string;
  readonly line: number;
}

export interface WasmCheck {
  readonly files: readonly WasmFacts[];
  readonly problems: readonly WasmProblem[];
}

/** `root` is the plugin directory's absolute path: a missing file is named by its absolute path. */
export async function checkWasmFiles(
  files: PluginFiles,
  named: readonly ScannedWasm[],
  where: { readonly root: string; readonly signal?: AbortSignal | undefined },
): Promise<WasmCheck> {
  const read = await Promise.all(named.map((one) => files.readBytes(one.path, where.signal)));
  const found: WasmFacts[] = [];
  const problems: WasmProblem[] = [];
  named.forEach(({ path, line }, index) => {
    const bytes = read[index];
    if (bytes === undefined) {
      const absolute = path.startsWith('/') ? path : `${where.root.replace(/\/+$/, '')}/${path}`;
      problems.push({ severity: 'error', message: `${absolute}: no such file`, line });
      return;
    }
    const facts = { path, bytes: bytes.length, sha256: sha256Hex(bytes) };
    const entries = readWasm(bytes);
    if ('at' in entries) {
      const message = `${path}: WebAssembly.Module doesn't parse at byte ${entries.at}: ${entries.reason}`;
      problems.push({ severity: 'warning', message, line });
      found.push(facts);
      return;
    }
    found.push({ ...facts, ...entries });
  });
  return { files: found, problems };
}

/** The validator's lines of one module's files, led by the module as `modules` writes it. */
export function wasmLines(written: string, facts: readonly WasmFacts[]): string[] {
  const listed = (items: readonly string[]): string =>
    items.length === 0 ? 'nothing' : items.join(', ');
  return facts.flatMap((file) => [
    `  ❯ ${written} wasm ${file.path}: ${file.bytes} bytes, sha256 ${file.sha256}`,
    ...(file.imports === undefined
      ? []
      : [`  ❯ ${written} wasm ${file.path} imports: ${listed(file.imports)}`]),
    ...(file.exports === undefined
      ? []
      : [`  ❯ ${written} wasm ${file.path} exports: ${listed(file.exports)}`]),
  ]);
}
