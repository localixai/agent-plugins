/**
 * Reading a JSON file of a plugin: the bytes come through the `PluginFiles` port, and the core
 * parses them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/read_json.rs`, docs/RUST.md).
 */
import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/read-json');

/** A JSON file's value, or the reason it cannot be used; `missing` when there was no text to parse. */
export type JsonRead =
  | { readonly value: unknown }
  | { readonly problem: string; readonly missing?: true };

/** The value of a JSON text, or the parser's reason it is not JSON; never a throw. */
export const parseJson = (text: string): JsonRead => call('parseJson', text);

/** The message a file that is no JSON gives (plugins/troubleshooting.md; plugins/create-marketplace.md): `Invalid JSON syntax: <reason>`. */
export const invalidJsonSyntax = (problem: string): string => call('invalidJsonSyntax', problem);

/** Reads and parses one file of the plugin; a missing file or invalid JSON is a `problem`, never a throw. */
export const readJson = (
  files: PluginFiles,
  path: string,
  signal?: AbortSignal,
): Promise<JsonRead> => call.async('readJson', files, path, signal);

/** What Claude Code's JSON parser says of a file that opens with a UTF-8 byte order mark. */
export const BOM_PARSE_ERROR: string = call('BOM_PARSE_ERROR');

/** Whether the bytes open with a UTF-8 byte order mark. */
export const opensWithBom = (bytes: Uint8Array): boolean => call('opensWithBom', bytes);

/**
 * Like `readJson`, but read as bytes so a byte order mark `readText` may drop is seen: a file that
 * opens with one is not JSON (`BOM_PARSE_ERROR`). For `hooks/hooks.json` and the hooks files a
 * manifest names, which load no hooks with a BOM (spec §2.1 item 2, J2.3; load oracle 2.1.319 and
 * 2.1.322: Hooks (0)). One read either way.
 */
export const readJsonWithoutBom = (
  files: PluginFiles,
  path: string,
  signal?: AbortSignal,
): Promise<JsonRead> => call.async('readJsonWithoutBom', files, path, signal);
