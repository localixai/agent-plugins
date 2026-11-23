/**
 * Which mod a pane is, and the Desktop app's widget.
 * Agent Plugins Extended §10.25.1 ("One pane exists per id; opening an id the mod has open retitles
 * it, and opening an id another mod has open MUST reject (2.1.330)"; `placement: 'widget'` "asks the
 * Desktop app to draw the pane as a widget (`Pane` props then say `placement: 'widget'`); a surface
 * without widgets opens it as without the field, and any other value is ignored",
 * claude-code/index.d.ts:9478); §10.9 (`Pane` `placement` is `dock`, `inline`, or `widget` in the
 * Desktop app); §10.13.3 (`e.plugin` is set on `ui.render` at a mod's `Pane`).
 */
import { expect, test } from 'bun:test';
import { uiRules } from '../../../src/mods/ui/index.ts';
import { apiHarness } from '../../support/mods-api.ts';
import { fn } from '../../support/mods-chain.ts';
import { rejection } from '../../support/rejection.ts';
import { fakeInterface, recordOf, uiOver } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

async function harness(surface: 'terminal' | 'desktop' = 'terminal') {
  const { iface } = fakeInterface({ origin: () => 'user', surface: () => surface });
  return apiHarness({ rules: uiRules, ui: uiOver(iface) });
}

spec('mods.api.$.ui', () => {
  test('opening an id the mod has open retitles it; an id another mod has open rejects', async () => {
    const h = await harness();
    const other = await h.mod('other-mod', 'user', []);
    await h.api.ui.open({ id: 'board', title: 'One' });
    await h.api.ui.open({ id: 'board', title: 'Two' });
    expect(h.ui?.panes.list().map((pane) => [pane.plugin, pane.request.title])).toEqual([
      ['my-mod', 'Two'],
    ]);
    expect(await rejection(other.api.ui.open({ id: 'board' }))).toBe(
      'ui.open: the pane board is open in another mod',
    );
    expect(h.ui?.panes.list().map((pane) => pane.plugin)).toEqual(['my-mod']);
  });

  test("open takes placement: 'widget' and ignores any other placement", async () => {
    const h = await harness('desktop');
    await h.api.ui.open({ id: 'w', placement: 'widget' });
    await h.api.ui.open({ id: 'x', placement: 'floating' });
    expect(h.ui?.panes.list().map((pane) => pane.request.placement)).toEqual(['widget', undefined]);
  });
});

spec('mods.site.Pane', () => {
  test("a widget pane's props say placement: 'widget' in the Desktop app, and the pane names its mod", async () => {
    const h = await harness('desktop');
    const seen: unknown[] = [];
    await h.mod('my-mod', 'user', [fn('ui.render', (_$, e, next) => (seen.push(e), next(e)))]);
    await h.api.ui.open({ id: 'w', placement: 'widget' });
    await h.ui?.draw({
      site: 'Pane',
      surface: 'desktop',
      requestId: 'w',
      props: { placement: 'dock' },
    });
    expect(seen).toMatchObject([{ plugin: 'my-mod', props: { placement: 'widget' } }]);
  });

  test('a surface without widgets draws the pane as without the field', async () => {
    const h = await harness('terminal');
    const seen: unknown[] = [];
    await h.mod('my-mod', 'user', [fn('ui.render', (_$, e, next) => (seen.push(e), next(e)))]);
    await h.api.ui.open({ id: 'w', placement: 'widget' });
    await h.ui?.draw({
      site: 'Pane',
      surface: 'terminal',
      requestId: 'w',
      props: { placement: 'inline' },
    });
    expect(recordOf(recordOf(seen[0]).props).placement).toBe('inline');
  });
});
