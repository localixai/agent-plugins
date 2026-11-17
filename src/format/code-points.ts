/**
 * The length of a text in Unicode code points, the unit the specification counts a name or an
 * option label in (spec §3.4.2, §4.3): a character outside the Basic Multilingual Plane is one,
 * not the two UTF-16 units `String.length` gives.
 */
export function codePointLength(text: string): number {
  let count = 0;
  for (const _point of text) count += 1;
  return count;
}
