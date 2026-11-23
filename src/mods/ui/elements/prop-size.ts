import { MOD_LIMITS } from '../../../protocol/mods-limits.ts';

interface Frame {
  readonly value: unknown;
  /** Lists and objects this value sits in, itself included when it is one. */
  readonly depth: number;
}

/** Bytes (an `Image`'s `source`) cross as one value, never as a list of numbers (judgment). */
const isNested = (value: unknown): value is object =>
  typeof value === 'object' &&
  value !== null &&
  !ArrayBuffer.isView(value) &&
  !(value instanceof ArrayBuffer);

/**
 * Why a tree counted with its prop values is refused, or `undefined` (reference.md, "Limits": "Depth
 * and size of a tree counted with its prop values", from v2.1.326): `nested more than 100 lists and
 * objects deep`, or `more than 1000000 values once written out`. A level is one array or object, a
 * scalar 0 levels (Judgment J10.10); every value counts once, a list or object as one beside what it
 * holds; bytes are one value (spec/judgments.md). Walked without recursion, and a value met twice (a cycle) counts as written out again until
 * the size limit stops it.
 */
export function propSizeProblem(tree: unknown): string | undefined {
  const stack: Frame[] = [{ value: tree, depth: isNested(tree) ? 1 : 0 }];
  let values = 0;
  for (let frame = stack.pop(); frame; frame = stack.pop()) {
    values += 1;
    if (values > MOD_LIMITS.propValues) {
      return `more than ${MOD_LIMITS.propValues} values once written out`;
    }
    if (frame.depth > MOD_LIMITS.propDepth) {
      return `nested more than ${MOD_LIMITS.propDepth} lists and objects deep`;
    }
    if (!isNested(frame.value)) continue;
    for (const inner of Object.values(frame.value)) {
      stack.push({ value: inner, depth: frame.depth + (isNested(inner) ? 1 : 0) });
    }
  }
  return undefined;
}
