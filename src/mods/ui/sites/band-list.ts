import { isRecord } from '../../../protocol/is-record.ts';

/** The `rows` of an `AbovePromptList` event, or `undefined` for any other event. */
function rowsOf(e: unknown): readonly unknown[] | undefined {
  if (!isRecord(e) || e.component !== 'AbovePromptList' || !isRecord(e.props)) return undefined;
  const { rows } = e.props;
  return Array.isArray(rows) ? rows : [];
}

/** Two values as data: the same scalars, the same keys holding the same values. */
export function sameValue(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length &&
    keys.every((key) => sameValue(Reflect.get(a, key), Reflect.get(b, key)))
  );
}

/**
 * Why the `AbovePromptList` event a hook passes to `next` is refused (§10.9 item 4): "a rewrite MUST
 * hold only rows it received, unchanged, each at most once, or the hook fails", rows "compared by
 * value: an unchanged copy passes, one changed or repeated fails the hook" (claude-code/index.d.ts:13628
 * of 2.1.330). The wording is the kit's.
 */
export function rowsChangeProblem(original: unknown, changed: unknown): string | undefined {
  const given = rowsOf(original);
  if (given === undefined) return undefined;
  const left = [...given];
  for (const row of rowsOf(changed) ?? []) {
    const at = left.findIndex((candidate) => sameValue(candidate, row));
    if (at < 0) {
      return 'the rows of the band hold only the rows given to the hook, unchanged, each at most once';
    }
    left.splice(at, 1);
  }
  return undefined;
}
