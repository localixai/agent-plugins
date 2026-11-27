/**
 * Bytes as the base64 the boundary carries them in (`wire.ts`, `{"$bytes": …}`), padded, both ways:
 * the runtime's own codec where it has one, otherwise one that never builds a string a byte at a time.
 */

/**
 * The runtime's own base64 (Bun, newer Node and browsers); absent on older Node. Cast because the
 * TypeScript lib does not declare them yet: each is read as possibly missing.
 */
const nativeToBase64 = (Uint8Array.prototype as { toBase64?: (this: Uint8Array) => string })
  .toBase64;
const nativeFromBase64 = (Uint8Array as { fromBase64?: (text: string) => Uint8Array }).fromBase64;

/** Bytes per `String.fromCharCode` call: large enough to be fast, small enough for the stack. */
const CHUNK = 0x8000;

export const toBase64 = (bytes: Uint8Array): string => {
  if (nativeToBase64) return nativeToBase64.call(bytes);
  let binary = '';
  for (let at = 0; at < bytes.length; at += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(at, at + CHUNK));
  }
  return btoa(binary);
};

export const fromBase64 = (text: string): Uint8Array => {
  if (nativeFromBase64) return nativeFromBase64(text);
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
};
