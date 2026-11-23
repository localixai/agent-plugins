/**
 * The `Backdrop` render site, raised for the session's backdrop.
 * plugins/mods/reference.md § Render sites (`Backdrop`: `e.props` `scene`, `plugin`, `glass`,
 * `overrideColorScheme`, "Before v2.1.323, `scene` was `id` and `overrideColorScheme` was
 * `colorScheme`"; `e.requestId` `backdrop`; "`desktop` and `mobile` surfaces. Your hook runs when the
 * Desktop or mobile app asks to draw a backdrop"); plugins/mods/interface.md § Put a backdrop behind
 * the conversation (the example's hook: "This hook runs for every mod's backdrop, and another mod
 * can use the same id", so it checks `e.props.plugin` and `e.props.scene`). When the app asks is the
 * host's: the kit builds the event from the backdrop `$.theme.set` set. What the mobile app draws
 * there is a judgment (tests/mods/ui/mobile.test.ts).
 */
import { expect, test } from 'bun:test';
import { backdropRender } from '../../../src/mods/ui/index.ts';
import { fn } from '../../support/mods-chain.ts';
import { recordOf, text, uiChain } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

const NIGHT_SKY = { scene: 'night-sky', plugin: 'night-sky', overrideColorScheme: 'dark' } as const;
const core = (e: unknown) => Promise.resolve({ type: 'engine', ref: e });

spec('mods.site.Backdrop', () => {
  test("the event holds the backdrop's fields as e.props and backdrop as e.requestId", () => {
    const glass = { blur: 14, opacity: 0.55 };
    expect(backdropRender({ ...NIGHT_SKY, glass }, 'desktop')).toEqual({
      event: {
        component: 'Backdrop',
        surface: 'desktop',
        props: { ...NIGHT_SKY, glass },
        requestId: 'backdrop',
      },
    });
  });

  test('the mobile app raises the site too, and e.surface names it', () => {
    // e.props always carries glass: the default glass when the backdrop names none (RenderPropsOf Backdrop).
    expect(backdropRender(NIGHT_SKY, 'mobile')).toEqual({
      event: {
        component: 'Backdrop',
        surface: 'mobile',
        props: { ...NIGHT_SKY, glass: { blur: 16, opacity: 0.6 } },
        requestId: 'backdrop',
      },
    });
  });

  test('the terminal and the VS Code extension draw no backdrop', () => {
    expect(backdropRender(NIGHT_SKY, 'terminal')).toEqual({
      problem: 'Backdrop is not raised in the terminal app',
    });
    expect('problem' in backdropRender(NIGHT_SKY, 'vscode')).toBe(true);
  });

  test("a hook draws its own backdrop and hands on another mod's", async () => {
    const h = uiChain();
    h.mod('night-sky', 'user', [
      fn('ui.render', (_$, e, next) => {
        const props = recordOf(e.props);
        if (props.plugin !== 'night-sky' || props.scene !== 'night-sky') return next(e);
        return text('Night sky');
      }),
    ]);
    const fire = async (plugin: string) => {
      const built = backdropRender({ ...NIGHT_SKY, plugin }, 'desktop');
      if (!('event' in built)) throw new Error(built.problem);
      return h.fire('ui.render', built.event, { core });
    };
    expect(await fire('night-sky')).toEqual(text('Night sky'));
    expect(await fire('other')).toMatchObject({ ref: { props: { plugin: 'other' } } });
  });
});

test('with no backdrop set there is nothing to draw (judgment)', () => {
  expect(backdropRender(undefined, 'desktop')).toEqual({ problem: 'the session has no backdrop' });
});
