/**
 * `$.theme`: the session's one theme. Agent Plugins Extended §10.25.6 ("The backdrop": the fields,
 * whether it is drawn, one theme for each session) and §10.25.7 ("The colours of the Desktop app",
 * "another `$.theme` call"); reference.md §10.7.1 (`$.theme`: `set`, `clear`), §10.12 Limits (the
 * `scene` of a backdrop is a name); J10.113 (the newer attributes are checked and passed as given).
 * Declarations 2.1.330: `ThemeSetArgs` (claude-code/index.d.ts:17320), `ThemeBackdrop` (:16859),
 * `ThemeGround` (:17128), `ThemeColors` (:16980). Whether an app draws backdrops is the host's
 * (the `theme` port).
 */
import { describe, expect, test } from 'bun:test';
import { MODS_API } from '../../../src/protocol/mods-api-names.ts';
import type { ModTheme, SessionTheme } from '../../../src/mods/api/index.ts';
import { sceneBackdrop } from '../../../src/mods/api/index.ts';
import { THEME_CLEARED } from '../../../src/mods/api/theme.ts';

import { apiHarness, expectMethods } from '../../support/mods-api.ts';
import { fn } from '../../support/mods-chain.ts';
import { rejection } from '../../support/rejection.ts';
import { spec } from '../../support/spec.ts';

/** The default glass a scene that names none carries (`ThemeBackdropGlass`). */
const GLASS = { blur: 16, opacity: 0.6 };

const SECTION = 'mods.interface.Put a backdrop behind the conversation';
const COLORS = {
  v: 1,
  label: 'Harbor',
  inputs: { light: { 'surface-0': '#ffffff' }, dark: { 'surface-0': '#101418' } },
} as const;

/** A theme port whose apps draw backdrops (or not, with `reason`), recording each change. */
function theme(reason?: string) {
  const changes: (SessionTheme | undefined)[] = [];
  const port: ModTheme = {
    drawn: () =>
      Promise.resolve(reason === undefined ? { isDrawn: true } : { isDrawn: false, reason }),
    changed: (next) => void changes.push(next),
  };
  return { port, changes };
}

async function setup(reason?: string, plugin = 'night-sky') {
  const { port, changes } = theme(reason);
  const h = await apiHarness({ plugin, ports: { theme: port } });
  return { h, changes, current: () => h.shared.registries.theme.current() };
}

const scene = (name: string, plugin = 'night-sky') => ({
  plugin,
  backdrop: { scene: name, glass: GLASS },
});

spec('mods.api.$.theme', () => {
  test('$.theme has set and clear, which reach the host', async () => {
    const { h, changes } = await setup();
    expectMethods(h.api.theme, MODS_API.theme);
    await h.api.theme.set({ backdrop: { scene: 'night-sky' } });
    await h.api.theme.clear();
    expect(changes).toEqual([scene('night-sky'), undefined]);
  });

  test('each call is an event a hook of another mod sees and can rewrite', async () => {
    const { h, current } = await setup();
    const seen: unknown[] = [];
    await h.mod('other', 'prepend', [
      fn('theme.set', (_$, e, next) => {
        seen.push(e);
        return next({ backdrop: { scene: 'dawn' }, overrideColorScheme: 'light' });
      }),
      fn('theme.clear', (_$, e, next) => (seen.push('clear'), next(e))),
    ]);
    await h.api.theme.set({ backdrop: { scene: 'night-sky' }, overrideColorScheme: 'dark' });
    expect(current()).toEqual({ ...scene('dawn'), overrideColorScheme: 'light' });
    await h.api.theme.clear();
    expect(seen).toEqual([
      { backdrop: { scene: 'night-sky' }, overrideColorScheme: 'dark' },
      'clear',
    ]);
  });
});

spec(`${SECTION}.The backdrop fields`, () => {
  test('a scene is a name with glass { blur, tint, opacity } (defaults filled in) or false; overrideColorScheme is beside the backdrop', async () => {
    const { h, current } = await setup();
    await h.api.theme.set({ backdrop: { scene: 'night-sky' }, overrideColorScheme: 'dark' });
    expect(current()).toEqual({ ...scene('night-sky'), overrideColorScheme: 'dark' });
    await h.api.theme.set({ backdrop: { scene: 'lagoon', glass: { blur: 14, opacity: 0.55 } } });
    expect(current()).toEqual({
      plugin: 'night-sky',
      backdrop: { scene: 'lagoon', glass: { blur: 14, opacity: 0.55 } },
    });
    await h.api.theme.set({ backdrop: { scene: 'veil', glass: { tint: '#102030' } } });
    expect(current()?.backdrop).toEqual({
      scene: 'veil',
      glass: { blur: 16, tint: '#102030', opacity: 0.6 },
    });
    await h.api.theme.set({ backdrop: { scene: 'day_1', glass: false } });
    expect(current()?.backdrop).toEqual({ scene: 'day_1', glass: false });
  });

  test('a still ground is { color }, { gradient } or { vector }, kept as given and drawn by no hook', async () => {
    const { h, current } = await setup();
    const grounds = [
      { color: 'surface-1' },
      { gradient: { stops: ['#000', '#fff'], angle: 90, shape: 'linear' } },
      { vector: { paths: ['M0 0L1 1'], viewBox: [0, 0, 1, 1] } },
    ];
    for (const backdrop of grounds) {
      await h.api.theme.set({ backdrop });
      expect(current()).toEqual({ plugin: 'night-sky', backdrop });
      expect(sceneBackdrop(current())).toBeUndefined();
    }
  });

  test('the pre-2.1.323 spellings, and a backdrop of no kind or two, are refused', async () => {
    const { h, current } = await setup();
    await h.api.theme.set({ backdrop: { scene: 'day_1' } });
    for (const wrong of [
      { backdrop: {} },
      { backdrop: { id: 'night-sky' } },
      { backdrop: { scene: 'night-sky', colorScheme: 'dark' }, colorScheme: 'dark' },
      { backdrop: { scene: 'night sky' } },
      { backdrop: { scene: 'a'.repeat(65) } },
      { backdrop: { scene: 'night-sky', color: 'surface-0' } },
      { backdrop: { color: 'surface-0', glass: false } },
      { backdrop: { scene: 'night-sky' }, overrideColorScheme: 'dim' },
      { backdrop: { scene: 'night-sky', glass: true } },
      { backdrop: { scene: 'night-sky', glass: { blur: 65 } } },
      { backdrop: { scene: 'night-sky', glass: { opacity: 1.5 } } },
      { backdrop: { scene: 'night-sky', glass: { tint: 'surface' } } },
    ]) {
      expect(await rejection(h.api.theme.set(wrong))).toStartWith('$.theme.set: ');
    }
    expect(current()).toEqual(scene('day_1'));
  });

  test("the Backdrop site's props are the scene, plugin, glass and overrideColorScheme", async () => {
    const { h, current } = await setup();
    await h.api.theme.set({ backdrop: { scene: 'night-sky' }, overrideColorScheme: 'dark' });
    expect(sceneBackdrop(current())).toEqual({
      scene: 'night-sky',
      plugin: 'night-sky',
      glass: GLASS,
      overrideColorScheme: 'dark',
    });
  });
});

spec(`${SECTION}.Whether the backdrop is drawn`, () => {
  test('set resolves { isDrawn: true, requestId } when an app connected to the session says it draws backdrops', async () => {
    const { h } = await setup();
    expect(await h.api.theme.set({ backdrop: { scene: 'night-sky' } })).toEqual({
      isDrawn: true,
      requestId: 'backdrop',
    });
  });

  test('otherwise isDrawn is false and reason says why', async () => {
    const { h, current } = await setup('no app that draws backdrops is connected');
    expect(await h.api.theme.set({ backdrop: { scene: 'night-sky' } })).toEqual({
      isDrawn: false,
      reason: 'no app that draws backdrops is connected',
    });
    expect(current()).toEqual(scene('night-sky'));
  });
});

spec(`${SECTION}.One theme for each session`, () => {
  test('a set replaces all of the theme, whichever mod set it, and what it leaves out is gone', async () => {
    const { h, changes, current } = await setup();
    const other = await h.mod('other', 'user', []);
    await h.api.theme.set({ backdrop: { scene: 'night-sky' }, colors: COLORS });
    await other.api.theme.set({ overrideColorScheme: 'light' });
    expect(current()).toEqual({ plugin: 'other', overrideColorScheme: 'light' });
    const cleared = await h.api.theme.clear();
    expect(cleared).toMatchObject({ reason: 'the theme is cleared' });
    expect(current()).toBeUndefined();
    expect(changes).toEqual([
      { ...scene('night-sky'), colors: COLORS },
      { plugin: 'other', overrideColorScheme: 'light' },
      undefined,
    ]);
  });

  test('it names at least one attribute, and an attribute the build does not know is refused by name', async () => {
    const { h, current } = await setup();
    expect(await rejection(h.api.theme.set({}))).toStartWith('$.theme.set: ');
    expect(await rejection(h.api.theme.set({ wallpaper: 'x' }))).toContain('wallpaper');
    expect(current()).toBeUndefined();
  });

  test('a mod that unloads takes its own theme with it; unloading another mod leaves it', async () => {
    const { h, changes, current } = await setup();
    const other = await h.mod('other', 'user', []);
    await h.api.theme.set({ backdrop: { scene: 'night-sky' } });
    other.dispose();
    expect(current()).toEqual(scene('night-sky'));
    h.handle.dispose();
    expect(current()).toBeUndefined();
    expect(changes).toEqual([scene('night-sky'), undefined]);
  });
});

spec('mods.interface.Fix missing mod colors in the Desktop app.Another $.theme call', () => {
  test("any mod's later set or clear removes the colours and backdrop an earlier call set", async () => {
    const { h, current } = await setup();
    const other = await h.mod('other', 'user', []);
    await h.api.theme.set({ backdrop: { scene: 'night-sky' }, colors: COLORS });
    await other.api.theme.set({ backdrop: { color: 'surface-2' } });
    expect(current()).toEqual({ plugin: 'other', backdrop: { color: 'surface-2' } });
    await h.api.theme.set({ colors: COLORS });
    await other.api.theme.clear();
    expect(current()).toBeUndefined();
  });
});

describe('judgments', () => {
  test('clear resolves { isDrawn: false, reason: "the theme is cleared" }, also with no theme set, and tells the host only when one was', async () => {
    const { h, changes } = await setup();
    expect(await h.api.theme.clear()).toEqual({ isDrawn: false, reason: THEME_CLEARED });
    expect(changes).toEqual([]);
  });

  test('J10.113: colors, sound and style are checked for their shape and passed as given; a bad one rejects before any hook sees it', async () => {
    const { h, current } = await setup();
    const seen: unknown[] = [];
    await h.mod('other', 'prepend', [fn('theme.set', (_$, e, next) => (seen.push(e), next(e)))]);
    const sound = { cues: { done: { tone: 'soft' } } };
    const style = { AssistantMessage: { color: 'text-primary' } };
    await h.api.theme.set({ colors: COLORS, sound, style });
    expect(current()).toEqual({ plugin: 'night-sky', colors: COLORS, sound, style });
    for (const wrong of [{ colors: { ...COLORS, v: 2 } }, { colors: { v: 1 } }, { sound: 3 }]) {
      expect(await rejection(h.api.theme.set(wrong))).toStartWith('$.theme.set: ');
    }
    expect(seen).toHaveLength(1);
  });
});
