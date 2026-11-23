/**
 * The band above the prompt, one row for each mod.
 * Agent Plugins Extended §10.21 item 3 ("From 2.1.323 each mod that draws there gets its own row,
 * and the client MUST stack the rows from the top in the order mods run in"; "one mod's tree no
 * longer replaces another's"); §10.9 (`AbovePrompt`, one mod's row, `e.plugin` naming the mod;
 * `AbovePromptList`, `rows` one `{ plugin, tree }` for each mod's row that settled and fits, in chain
 * order, plus the `AbovePrompt` props; item 4, "The band's rows": a row past a tree's limits
 * "counted across the rows, kept from the top, stacking adding one level of depth" draws nothing
 * while the other rows draw; a hook on `AbovePromptList` MAY hide or reorder the rows and wrap
 * them, and "a rewrite MUST hold only rows it received, unchanged, each at most once, or the hook
 * fails"); §10.11 ("The rows above the prompt, stacked"); §10.44.1 and §10.44.3 (troubleshoot.md,
 * "Another mod's band shows and yours doesn't"; the line `hooks: ui.render (AbovePrompt) refused: a
 * row past the limits of the band; nothing was drawn`, "despite that ending, the client MUST still
 * draw the rows that fit").
 */
import { expect, test } from 'bun:test';
import { isEngineRef, uiRules, type ElementNode } from '../../../src/mods/ui/index.ts';
import { apiHarness } from '../../support/mods-api.ts';
import { fn, type FnHook } from '../../support/mods-chain.ts';
import { fakeInterface, node, recordOf, text, uiOver } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

const BAND = { component: 'AbovePrompt' } as const;
const T = "mods.troubleshoot.Another mod's band shows and yours doesn't";
const props = { hasSurvey: false, isCollapsed: false, isWorking: true, maxRows: 4 };

async function band() {
  return apiHarness({ rules: uiRules, ui: uiOver(fakeInterface().iface) });
}

/** A Box holding `count` text children: `count + 1` elements and text children. */
const wide = (count: number): ElementNode =>
  node('Box', { children: Array.from({ length: count }, () => 'x') });

type Next = Parameters<FnHook>[2];

/**
 * A hook on the stack: "heard by a hook naming it or a bare `on("*")`" (claude-code/index.d.ts:13612);
 * the bare wildcard hears every row too, and passes there.
 */
const onList = (hook: (e: Readonly<Record<string, unknown>>, next: Next) => unknown) =>
  fn('*', (_$, e, next) =>
    recordOf(e).component === 'AbovePromptList' ? hook(recordOf(e), next) : next(e),
  );

/** `e` with its `rows` rewritten. */
const withRows = (
  e: Readonly<Record<string, unknown>>,
  rewrite: (rows: unknown[]) => unknown[],
) => {
  const given = recordOf(e.props).rows;
  return {
    ...e,
    props: { ...recordOf(e.props), rows: rewrite(Array.isArray(given) ? given : []) },
  };
};

const plugins = (rows: readonly { readonly plugin: string }[]) => rows.map((row) => row.plugin);

spec(['mods.site.AbovePrompt', 'mods.interface.Pick where to draw'], () => {
  test("each mod's hook draws its own row, asked with e.plugin naming it; the rows stack in chain order", async () => {
    const h = await band();
    const seen: unknown[] = [];
    await h.mod('first', 'user', [
      fn('ui.render', (_$, e) => (seen.push(recordOf(e).plugin), text('one')), { matcher: BAND }),
    ]);
    await h.mod('second', 'user', [
      fn('ui.render', (_$, e) => (seen.push(recordOf(e).plugin), text('two')), { matcher: BAND }),
    ]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal', props }, ['first', 'second']);
    expect(seen).toEqual(['first', 'second']);
    expect(drawn?.rows).toEqual([
      { plugin: 'first', tree: text('one') },
      { plugin: 'second', tree: text('two') },
    ]);
    expect(drawn?.lines).toEqual([]);
  });

  test('a mod whose hook passes draws no row, and a mod with no ui.render hook is not asked', async () => {
    const h = await band();
    await h.mod('passer', 'user', [fn('ui.render', (_$, e, next) => next(e), { matcher: BAND })]);
    await h.mod('drawer', 'user', [fn('ui.render', () => text('mine'), { matcher: BAND })]);
    await h.mod('silent', 'user', []);
    const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['passer', 'drawer', 'silent']);
    expect(drawn && plugins(drawn.rows)).toEqual(['drawer']);
  });
});

spec('mods.site.AbovePromptList', () => {
  test('the stack is asked once with rows and the AbovePrompt props; left alone, Claude Code stacks the rows', async () => {
    const h = await band();
    const seen: unknown[] = [];
    await h.mod('first', 'user', [fn('ui.render', () => text('one'), { matcher: BAND })]);
    await h.mod('lister', 'user', [onList((e, next) => (seen.push(e), next(e)))]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal', props }, ['first', 'lister']);
    expect(seen).toMatchObject([
      { component: 'AbovePromptList', props: { ...props, rows: [{ plugin: 'first' }] } },
    ]);
    expect(drawn?.drawn).toMatchObject({ engine: { type: 'engine' } });
  });

  test('a hook may hide or reorder the rows it is handed', async () => {
    const h = await band();
    await h.mod('a', 'user', [fn('ui.render', () => text('a'), { matcher: BAND })]);
    await h.mod('b', 'user', [fn('ui.render', () => text('b'), { matcher: BAND })]);
    await h.mod('sorter', 'user', [
      onList((e, next) => next(withRows(e, (rows) => rows.toReversed()))),
    ]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['a', 'b', 'sorter']);
    const settled = drawn && 'engine' in drawn.drawn ? drawn.drawn.engine : undefined;
    expect(isEngineRef(settled)).toBe(true);
    expect(recordOf(recordOf(recordOf(settled).ref).props).rows).toMatchObject([
      { plugin: 'b' },
      { plugin: 'a' },
    ]);
    expect(h.reports).toEqual([]);
  });

  test('a rewrite that holds a row it was not handed, a changed row or a row twice fails the hook', async () => {
    const rewrites: readonly ((rows: readonly unknown[]) => unknown[])[] = [
      (rows) => [...rows, { plugin: 'forged', tree: text('x') }],
      (rows) => rows.map((row) => ({ ...recordOf(row), tree: text('changed') })),
      (rows) => [...rows, ...rows],
    ];
    for (const rewrite of rewrites) {
      const h = await band();
      await h.mod('a', 'user', [fn('ui.render', () => text('a'), { matcher: BAND })]);
      await h.mod('cheat', 'user', [onList((e, next) => next(withRows(e, rewrite)))]);
      await h.ui?.drawBand({ surface: 'terminal' }, ['a', 'cheat']);
      expect(h.reports.map((report) => report.plugin)).toEqual(['cheat']);
      expect(h.reports[0]?.line).toContain('the rows of the band hold only the rows given');
    }
  });

  test('rows are compared by value: an unchanged copy passes', async () => {
    const h = await band();
    await h.mod('a', 'user', [fn('ui.render', () => text('a'), { matcher: BAND })]);
    await h.mod('copier', 'user', [
      onList((e, next) => next(withRows(e, (rows) => rows.map((row) => structuredClone(row))))),
    ]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['a', 'copier']);
    expect(h.reports).toEqual([]);
    const settled = drawn && 'engine' in drawn.drawn ? drawn.drawn.engine : undefined;
    expect(recordOf(recordOf(recordOf(settled).ref).props).rows).toEqual([
      { plugin: 'a', tree: text('a') },
    ]);
  });
});

spec([`${T}.Your tree doesn't validate`], () => {
  test("a row that does not validate is skipped with its line while the other mod's row stays", async () => {
    const h = await band();
    await h.mod('broken', 'user', [
      fn('ui.render', () => node('Text', { bogusProp: 1 }), { matcher: BAND }),
    ]);
    await h.mod('fine', 'user', [fn('ui.render', () => text('fine'), { matcher: BAND })]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['broken', 'fine']);
    expect(drawn && plugins(drawn.rows)).toEqual(['fine']);
    expect(h.reports.map((report) => report.line)).toEqual([
      'broken: ui.render hook skipped: returned the wrong shape (a tree AbovePrompt refuses: Text prop "bogusProp" is not allowed)',
    ]);
  });
});

spec(
  [
    'mods.limit.The rows above the prompt, stacked',
    `${T}.The rows together exceed a limit`,
    'mods.troubleshoot.A ui.render line says refused',
  ],
  () => {
    test('elements add up across the rows: the rows past the limit are left out with the band line, the rows that fit draw', async () => {
      const h = await band();
      await h.mod('a', 'user', [fn('ui.render', () => wide(12_000), { matcher: BAND })]);
      await h.mod('b', 'user', [fn('ui.render', () => wide(12_000), { matcher: BAND })]);
      await h.mod('c', 'user', [fn('ui.render', () => wide(10), { matcher: BAND })]);
      const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['a', 'b', 'c']);
      expect(drawn && plugins(drawn.rows)).toEqual(['a', 'c']);
      expect(drawn?.lines).toEqual([
        'hooks: ui.render (AbovePrompt) refused: a row past the limits of the band; nothing was drawn',
      ]);
    });

    test('stacking adds one level of depth: a row 32 levels deep is past the band', async () => {
      let deep = node('Box');
      for (let level = 0; level < 32; level += 1) deep = node('Box', { children: [deep] });
      const h = await band();
      await h.mod('deep', 'user', [fn('ui.render', () => deep, { matcher: BAND })]);
      const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['deep']);
      expect(drawn?.rows).toEqual([]);
      expect(drawn?.lines).toEqual([
        'ui.render (AbovePrompt) refused: a row past the limits of the band; nothing was drawn',
      ]);
    });
  },
);

spec(`${T}.Another mod hides or draws over your row`, () => {
  test('with neither line, the row the mod drew is missing because a hook on the stack hid it', async () => {
    const h = await band();
    await h.mod('mine', 'user', [fn('ui.render', () => text('mine'), { matcher: BAND })]);
    await h.mod('hider', 'user', [onList((e, next) => next(withRows(e, () => [])))]);
    const drawn = await h.ui?.drawBand({ surface: 'terminal' }, ['mine', 'hider']);
    expect(drawn && plugins(drawn.rows)).toEqual(['mine']);
    expect(drawn?.lines).toEqual([]);
    expect(h.reports).toEqual([]);
    const settled = drawn && 'engine' in drawn.drawn ? drawn.drawn.engine : undefined;
    expect(recordOf(recordOf(recordOf(settled).ref).props).rows).toEqual([]);
  });
});
