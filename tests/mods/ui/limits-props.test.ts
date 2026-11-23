/**
 * The depth and size of a tree counted with its prop values.
 * Agent Plugins Extended §10.11 (reference.md, "Limits": "Depth and size of a tree counted with its
 * prop values | refused when `nested more than 100 lists and objects deep` or with `more than
 * 1000000 values once written out` (from v2.1.326)"; a level is one array or object, a scalar 0
 * levels, Judgment J10.10). A tree over a limit is not drawn (§10.24).
 */
import { expect, test } from 'bun:test';
import { foldRender, sizeProblem } from '../../../src/mods/ui/index.ts';
import { node } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

/** A value nested `levels` lists deep: `[[[0]]]` is 3. */
function lists(levels: number): unknown {
  let value: unknown = 0;
  for (let level = 0; level < levels; level += 1) value = [value];
  return value;
}

/** A Svg-free `Box` whose `hover` prop carries `value` (the Box itself, its props and the value are three levels). */
const carrying = (value: unknown) => node('Box', { hover: value });

spec('mods.limit.Depth and size of a tree counted with its prop values', () => {
  test('a prop value nested within 100 lists and objects of the tree is drawn, one more is refused', () => {
    // the Box, its props and the hover value's own lists: 2 + 98 levels
    expect(sizeProblem(carrying(lists(98)))).toBeUndefined();
    expect(sizeProblem(carrying(lists(99)))).toBe('nested more than 100 lists and objects deep');
  });

  test('a tree of more than 1,000,000 values once written out is refused', () => {
    const many = Array.from({ length: 1_000_000 }, () => 1);
    expect(sizeProblem(carrying(many))).toBe('more than 1000000 values once written out');
    expect(sizeProblem(carrying(many.slice(10)))).toBeUndefined();
  });

  test('the refusal is a ui.render line naming the site, and nothing is drawn', () => {
    expect(foldRender('Pane', 'terminal', carrying(lists(99)))).toEqual({
      refused: 'nested more than 100 lists and objects deep',
      line: 'ui.render (Pane) refused: nested more than 100 lists and objects deep',
    });
  });
});
