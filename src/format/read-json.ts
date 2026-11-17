import type { PluginFiles } from '../ports/index.ts';

/** A JSON file's value, or the reason it cannot be used; `missing` when there was no text to parse. */
export type JsonRead =
  | { readonly value: unknown }
  | { readonly problem: string; readonly missing?: true };

/** The value of a JSON text, or the parser's reason it is not JSON; never a throw. */
export function parseJson(text: string): JsonRead {
  try {
    return { value: JSON.parse(text) };
  } catch (failure) {
    return { problem: failure instanceof Error ? failure.message : 'invalid JSON' };
  }
}

/** The message a file that is no JSON gives (plugins/troubleshooting.md; plugins/create-marketplace.md): `Invalid JSON syntax: <reason>`. */
export const invalidJsonSyntax = (problem: string): string => `Invalid JSON syntax: ${problem}`;

/** Reads and parses one file of the plugin; a missing file or invalid JSON is a `problem`, never a throw. */
export async function readJson(
  files: PluginFiles,
  path: string,
  signal?: AbortSignal,
): Promise<JsonRead> {
  const text = await files.readText(path, signal);
  return text === undefined
    ? { problem: 'the file could not be read', missing: true }
    : parseJson(text);
}

/** What Claude Code's JSON parser says of a file that opens with a UTF-8 byte order mark. */
export const BOM_PARSE_ERROR = "JSON Parse error: Unrecognized token ''";

const BOM = [0xef, 0xbb, 0xbf] as const;
const decoder = new TextDecoder();

/** Whether the bytes open with a UTF-8 byte order mark. */
export const opensWithBom = (bytes: Uint8Array): boolean =>
  BOM.every((byte, index) => bytes[index] === byte);

/**
 * Like `readJson`, but read as bytes so a byte order mark `readText` may drop is seen: a file that
 * opens with one is not JSON (`BOM_PARSE_ERROR`). For `hooks/hooks.json` and the hooks files a
 * manifest names, which load no hooks with a BOM (spec §2.1 item 2, J2.3; load oracle 2.1.319 and
 * 2.1.322: Hooks (0)). One read either way.
 */
export async function readJsonWithoutBom(
  files: PluginFiles,
  path: string,
  signal?: AbortSignal,
): Promise<JsonRead> {
  const bytes = await files.readBytes(path, signal);
  if (bytes === undefined) return { problem: 'the file could not be read', missing: true };
  return opensWithBom(bytes) ? { problem: BOM_PARSE_ERROR } : parseJson(decoder.decode(bytes));
}
