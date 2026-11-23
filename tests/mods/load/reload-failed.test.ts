/**
 * Agent Plugins Extended §10.41.7 (troubleshoot.md § `reload failed`, 2.1.330) and §10.8.2 item 3:
 * a save that breaks a hot-reloading mod writes one of two lines; the `mods active` line of
 * `/plugin` names the mod exactly when its last working version still runs.
 */
import { expect, test } from 'bun:test';
import { activeModsLine } from '../../../src/mods/load/index.ts';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

/** A mod whose module throws while `state.breaks` loads remain, and whose text `state.source` the folder serves. */
function editable() {
  const state = { breaks: 0, source: 'export function register(on) {}' };
  const register = registerOf(() => {
    if (state.breaks > 0) {
      state.breaks -= 1;
      throw new Error('it broke');
    }
  });
  const base = modPlugin(inline('first-mod'), register);
  const plugin = {
    ...base,
    files: {
      ...base.files,
      readText: (path: string) =>
        path === 'hooks/register.js' ? Promise.resolve(state.source) : base.files.readText(path),
    },
  };
  return { state, plugin, ...session({ 'first-mod': register }) };
}

spec('mods.troubleshoot.reload failed', () => {
  test('`reload failed, the previous version stays loaded: <reason>`: the last working version keeps running and /plugin names it', async () => {
    const { host, plugin, state } = editable();
    await host.load([plugin]);
    state.source = 'export function register(on) { ';
    const outcome = await host.reloadMod('first-mod');
    expect(outcome?.line).toStartWith(
      'first-mod: reload failed, the previous version stays loaded: ',
    );
    expect(activeModsLine(host.active())).toBe('1 mod active · first-mod');
  });

  test('`reload failed: <reason>` where no version stays: /plugin no longer names the mod', async () => {
    const { host, plugin, state } = editable();
    await host.load([plugin]);
    state.breaks = 2;
    const outcome = await host.reloadMod('first-mod');
    expect(outcome?.line).toBe(
      'first-mod: reload failed: hooks module did not load: hooks/register.js: it broke',
    );
    expect(activeModsLine(host.active())).toBeUndefined();
  });

  test('a later save that loads writes a `reloaded` line', async () => {
    const { host, plugin, state } = editable();
    await host.load([plugin]);
    state.source = 'export function register(on) { ';
    await host.reloadMod('first-mod');
    state.source = 'export function register(on) {}';
    const outcome = await host.reloadMod('first-mod');
    expect(outcome?.line).toStartWith('first-mod: reloaded');
  });

  test('a change that still fails when every plugin reloads unloads the version that stayed', async () => {
    const { host, plugin, state } = editable();
    await host.load([plugin]);
    state.source = 'export function register(on) { ';
    await host.reloadMod('first-mod');
    await host.reload();
    expect(activeModsLine(host.active())).toBeUndefined();
  });
});
