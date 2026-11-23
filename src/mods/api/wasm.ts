import type { ModsNamespaceApi } from '../types.ts';
import { call, parse, type Deps } from './call.ts';
import { z } from 'zod';

/**
 * `$.wasm.compile(source)` (Agent Plugins Extended §10.7.1, §10.35; claude-code/index.d.ts:4605,
 * v2.1.326): a string literal path of a file up to 64 MiB, or the binary's bytes, compiled into a
 * `WebAssembly.Module` to pass to `WebAssembly.instantiate`. Its event carries the bytes as
 * `{ base64, path? }`; the host's `wasm` port compiles them.
 */

const LABEL = '$.wasm.compile';
/** "a file of up to 64 MiB" (api.md, "Reach files, processes, and the network"; reference.md, `$.wasm`). */
const WASM_BYTES = 64 * 1024 * 1024;
const ABSOLUTE = /^(?:[A-Za-z]:[\\/]|[\\/])/;
const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
const sourceEvent = z.looseObject({ base64: z.string(), path: z.string().optional() });

/** Standard padded base64 of `bytes`. */
function base64Of(bytes: Uint8Array): string {
  let out = '';
  for (let at = 0; at < bytes.length; at += 3) {
    const [a = 0, b = 0, c = 0] = [bytes[at], bytes[at + 1], bytes[at + 2]];
    const word = (a << 16) | (b << 8) | c;
    const chars = [18, 12, 6, 0].map((shift) => ALPHABET.charAt((word >> shift) & 63));
    const kept = Math.min(4, bytes.length - at + 1);
    out += chars.slice(0, kept).join('') + '='.repeat(4 - kept);
  }
  return out;
}

/** The bytes of an `ArrayBuffer` or an `ArrayBufferView`, or `undefined` for anything else. */
function bytesOf(source: unknown): Uint8Array | undefined {
  if (source instanceof ArrayBuffer) return new Uint8Array(source);
  if (ArrayBuffer.isView(source)) {
    return new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
  }
  return undefined;
}

const overLimit = (what: string, bytes: number): Error =>
  new Error(`${LABEL}: ${what}: ${bytes} bytes is over the ${WASM_BYTES}-byte limit`);

/**
 * The file a path names. Judgment (spec/judgments.md): the kit does not know which file of the
 * module made the call, so a relative path resolves against the plugin's root, the directory of a
 * module at the root; the static analysis holds the path to a literal naming an existing file.
 */
async function readFile(deps: Deps, path: string): Promise<{ base64: string; path: string }> {
  const { locations, fs } = deps.ports;
  const full = ABSOLUTE.test(path)
    ? path
    : locations.joinPath(locations.pluginRoot(deps.plugin), path);
  const file = await fs.read(full, deps.signal, 'bytes');
  if (file === undefined) throw new Error(`${LABEL}: ${full}: no such file`);
  if (file.bytes > WASM_BYTES) throw overLimit(full, file.bytes);
  if (file.base64 === undefined) throw new Error(`${LABEL}: ${full}: the host read no bytes`);
  return { base64: file.base64, path: full };
}

async function sourceOf(deps: Deps, source: unknown): Promise<z.infer<typeof sourceEvent>> {
  if (typeof source === 'string') return readFile(deps, source);
  const bytes = bytesOf(source);
  if (bytes === undefined) throw new Error(`${LABEL}: takes a path or the module's bytes`);
  if (bytes.byteLength > WASM_BYTES) throw overLimit('the bytes', bytes.byteLength);
  return { base64: base64Of(bytes) };
}

export const wasmApi = (deps: Deps): ModsNamespaceApi<'wasm'> => ({
  compile: async (...args) =>
    call(deps, 'wasm.compile', await sourceOf(deps, args[0]), (e) =>
      deps.ports.wasm.compile(parse(LABEL, sourceEvent, e), deps.plugin, deps.signal),
    ),
});
