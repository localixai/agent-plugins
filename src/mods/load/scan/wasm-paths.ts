import type { CallExpression } from 'acorn';
import { stringOf } from './ast.ts';
import { joined } from './client-modules.ts';
import type { Found } from './found.ts';

/**
 * The files a module's `$.wasm.compile` calls name (Agent Plugins Extended §10.4.2: "a string literal
 * naming a file, relative to the calling file"; §10.4.3, the `wasm <path>:` lines; 2.1.326 on). A
 * call given bytes, or a path the scan cannot read, names no file: `$.wasm.compile` takes bytes too.
 */

/** One file a `$.wasm.compile` call names: its path from the plugin root and the line of the first call. */
export interface ScannedWasm {
  readonly path: string;
  readonly line: number;
}

/** Notes the literal path of a `$.wasm.compile` call, at the line it is first named. */
export function recordWasm(
  found: Found,
  name: string,
  call: CallExpression,
  lineOf: (offset: number) => number,
): void {
  if (name !== 'wasm.compile') return;
  const path = stringOf(call.arguments[0]);
  if (path !== undefined && !found.wasm.has(path)) found.wasm.set(path, lineOf(call.start));
}

/** The files, as paths from the plugin root: a relative path from the calling file's `directory`. */
export const wasmFilesOf = (found: Found, directory: string): ScannedWasm[] =>
  [...found.wasm].map(([path, line]) => ({
    path: path.startsWith('/') ? path : joined(directory, path),
    line,
  }));
