/**
 * A numeric environment variable (spec §3.6.1 item 2, J3.17): plain digits, scientific notation
 * (`2e3` is 2000) and digit separators (`64_000` is 64000), unless the variable's row says plain
 * digits only. A sign, a blank or any other character makes the value unreadable (judgment). A value that cannot be read is `undefined`, so the caller
 * keeps its default.
 */

const DIGITS = String.raw`\d+(?:_\d+)*`;
const NUMERIC = new RegExp(`^${DIGITS}(?:\\.${DIGITS})?(?:[eE][+-]?\\d+)?$`);
const PLAIN = /^\d+$/;

/** The number a numeric variable holds, or `undefined` when it holds none. */
export function readNumberEnv(text: string | undefined): number | undefined {
  if (text === undefined || !NUMERIC.test(text)) return undefined;
  const value = Number(text.replaceAll('_', ''));
  return Number.isFinite(value) ? value : undefined;
}

/** A whole number, `0` included, read as `readNumberEnv` reads it. */
export function wholeEnv(text: string | undefined): number | undefined {
  const value = readNumberEnv(text);
  return value !== undefined && Number.isSafeInteger(value) ? value : undefined;
}

/** A whole number above zero, read as `readNumberEnv` reads it. */
export function positiveWholeEnv(text: string | undefined): number | undefined {
  const value = wholeEnv(text);
  return value !== undefined && value > 0 ? value : undefined;
}

/** A whole number above zero in plain digits only, for a variable whose row says so. */
export function plainPositiveWholeEnv(text: string | undefined): number | undefined {
  if (text === undefined || !PLAIN.test(text)) return undefined;
  const value = Number(text);
  return Number.isSafeInteger(value) && value > 0 ? value : undefined;
}
