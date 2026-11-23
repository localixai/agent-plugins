/**
 * The elements and props the 2.1.330 declarations add.
 * Agent Plugins Extended §10.10 (reference.md, "Elements": `Progress` and `Shader` "in the 2.1.330
 * declarations' per-surface `Elements` tables ... a client MUST offer them where those tables do",
 * Judgment J10.24; "A `Shader` draws only in a `Backdrop`, at most one, at the root or in a `Box`;
 * anywhere else, or over anything but one `Svg`, the tree is refused"; `Text` `maxLines?` and
 * `wrap: 'clip'` "(as well as `end` and `middle`)", `Box` `overflowX?` (`visible` or `scroll`)).
 * The `Text` and `Box` props are on the rows `mods.element.Text` and `mods.element.Box`; `Progress`
 * and `Shader` have no requirement (J10.24), so their tests run outside `spec(...)`.
 */
import { expect, test } from 'bun:test';
import { defaultElementTable, treeProblem } from '../../../src/mods/ui/index.ts';
import { ELEMENT_NAMES } from '../../../src/protocol/mods-ui-names.ts';
import { MODS_REFERENCE } from '../../support/mods-reference.ts';
import { node, text } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

const svg = () => node('Svg', { source: '<svg/>', alt: 'An empty stage' });
const shader = (props: Record<string, unknown> = {}, children: unknown[] = [svg()]) =>
  node('Shader', { module: './beam.ts', ...props, children });

spec('mods.element.Text', () => {
  test('maxLines takes a whole number from 1', () => {
    expect(treeProblem('Pane', 'terminal', node('Text', { maxLines: 2, children: ['x'] }))).toBe(
      undefined,
    );
    expect(treeProblem('Pane', 'terminal', node('Text', { maxLines: 0, children: ['x'] }))).toBe(
      'a tree Pane refuses: Text prop "maxLines" must be a whole number from 1',
    );
  });

  test("wrap takes 'clip', 'end' and 'middle' beside the truncations", () => {
    for (const wrap of ['clip', 'end', 'middle', 'truncate-middle']) {
      expect(treeProblem('Pane', 'terminal', node('Text', { wrap, children: ['x'] })), wrap).toBe(
        undefined,
      );
    }
    expect(treeProblem('Pane', 'terminal', node('Text', { wrap: 'fold' }))).toBe(
      'a tree Pane refuses: Text prop "wrap" must be one of wrap, end, middle, truncate, truncate-start, truncate-middle, truncate-end, clip',
    );
  });
});

spec('mods.element.Box', () => {
  test("overflowX is 'visible' or 'scroll'", () => {
    expect(treeProblem('Pane', 'terminal', node('Box', { overflowX: 'scroll' }))).toBe(undefined);
    expect(treeProblem('Pane', 'terminal', node('Box', { overflowX: 'hidden' }))).toBe(
      'a tree Pane refuses: Box prop "overflowX" must be one of visible, scroll',
    );
  });
});

test("the elements beyond the documentation's table are the declarations-only Progress and Shader (J10.24)", () => {
  const documented = MODS_REFERENCE.elements.map((row) => row.name);
  expect(ELEMENT_NAMES.filter((name) => !documented.includes(name))).toEqual([
    'Progress',
    'Shader',
  ]);
});

test('Progress is offered on every surface and takes no children (J10.24)', () => {
  for (const surface of ['terminal', 'desktop', 'vscode', 'mobile'] as const) {
    expect(defaultElementTable(surface).Progress, surface).toBeDefined();
  }
  const bar = node('Progress', {
    label: 'Indexing',
    value: 412,
    max: 900,
    valueText: '412 of 900',
  });
  expect(treeProblem('Pane', 'terminal', bar)).toBe(undefined);
  expect(treeProblem('Pane', 'terminal', node('Progress', { label: 'x', children: ['y'] }))).toBe(
    'a tree Pane refuses: Progress takes no children',
  );
});

test('a Progress label is not blank (J10.24)', () => {
  expect(treeProblem('Pane', 'terminal', node('Progress', { label: '  ' }))).toBe(
    'a tree Pane refuses: Progress prop "label" must not be blank',
  );
  expect(treeProblem('Pane', 'terminal', node('Progress', {}))).toBe(
    'a tree Pane refuses: Progress prop "label" must not be blank',
  );
});

test('Shader is offered in the Desktop and mobile apps only (J10.24)', () => {
  expect(defaultElementTable('desktop').Shader).toBeDefined();
  expect(defaultElementTable('mobile').Shader).toBeDefined();
  expect(defaultElementTable('terminal').Shader).toBeUndefined();
  expect(defaultElementTable('vscode').Shader).toBeUndefined();
});

test('a Shader draws in a Backdrop, at the root or in a Box, over one Svg (J10.24)', () => {
  expect(treeProblem('Backdrop', 'desktop', shader({ values: { x: 0.5 } }))).toBe(undefined);
  expect(treeProblem('Backdrop', 'mobile', node('Box', { children: [shader()] }))).toBe(undefined);
});

test('a Shader anywhere else, or over anything but one Svg, refuses the tree (J10.24)', () => {
  expect(treeProblem('Pane', 'desktop', shader())).toBe(
    'a tree Pane refuses: Shader draws only in a Backdrop',
  );
  expect(treeProblem('Backdrop', 'desktop', node('Box', { children: [shader(), shader()] }))).toBe(
    'a tree Backdrop refuses: a Backdrop holds at most one Shader',
  );
  expect(treeProblem('Backdrop', 'desktop', shader({}, [svg(), svg()]))).toBe(
    'a tree Backdrop refuses: Shader holds exactly one Svg',
  );
  expect(treeProblem('Backdrop', 'desktop', shader({}, [text('x')]))).toBe(
    'a tree Backdrop refuses: Shader holds exactly one Svg',
  );
});

test('a Shader takes at most 8 finite values (J10.24)', () => {
  const nine = Object.fromEntries(Array.from({ length: 9 }, (_, i) => [`v${i}`, i]));
  expect(treeProblem('Backdrop', 'desktop', shader({ values: nine }))).toBe(
    'a tree Backdrop refuses: Shader prop "values" holds at most 8 finite numbers',
  );
  expect(treeProblem('Backdrop', 'desktop', shader({ values: { x: Infinity } }))).toBe(
    'a tree Backdrop refuses: Shader prop "values" holds at most 8 finite numbers',
  );
});
