/**
 * plugins/mods/interface.md (2.1.330) § "Remove mod colors from the Desktop app": "turn off the mod
 * first. A mod that's still on can set its colors again", then "Start a new session": for a mod you
 * installed, "disable or uninstall its plugin"; for one you put under `~/.claude/skills/`, "move its
 * directory ... to a place outside `~/.claude/skills/`". Agent Plugins Extended §10.25.7 item 4,
 * §10.25.6 item 3 (a mod that unloads takes its own theme with it), §10.8.3 (disabling unloads).
 * The colours mod is the page's `paper` (§ "Write a colors mod yourself"): it calls `$.theme.set`
 * in `session.start`.
 */
import { describe, expect, test } from 'bun:test';
import type { ModTheme, SessionTheme } from '../../../src/mods/api/index.ts';
import { fakePorts } from '../../../src/testing/mods/index.ts';
import { signal } from '../../support/host.ts';
import { installed, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { HOME, folderHostOf } from '../../support/scan.ts';
import { spec } from '../../support/spec.ts';

const COLORS = {
  v: 1,
  label: 'Paper',
  inputs: { light: { 'surface-0': '#efe6d3' }, dark: { 'surface-0': '#1f1a15' } },
} as const;

type Next = (e: unknown) => Promise<unknown>;
interface ThemeCaller {
  readonly theme: { set(args: unknown): Promise<unknown> };
}

/** The page's `register.js`: the colours are set when a session starts. */
const paper = registerOf((on) => {
  on('session.start', async ($: ThemeCaller, e: unknown, next: Next) => {
    await $.theme.set({ colors: COLORS });
    return next(e);
  });
});

/** A session whose app paints themes, recording each theme it is told about. */
function colorsSession() {
  const changes: (SessionTheme | undefined)[] = [];
  const theme: ModTheme = {
    drawn: () => Promise.resolve({ isDrawn: true }),
    changed: (next) => void changes.push(next),
  };
  const { host } = session({ paper }, { ports: { ...fakePorts().ports, theme } });
  const colors = () => changes.map((each) => each?.colors?.label);
  return { host, colors };
}

spec('mods.interface.Remove mod colors from the Desktop app.You installed the mod', () => {
  test('while its plugin is on the mod sets its colours again, on a reload as in each new session', async () => {
    const { host, colors } = colorsSession();
    await host.load([modPlugin(installed('paper'), paper)]);
    await host.reload();
    expect(colors()).toEqual(['Paper', undefined, 'Paper']);
  });

  test('disabling the plugin turns the mod off: its colours go and a reload does not set them again', async () => {
    const { host, colors } = colorsSession();
    await host.load([modPlugin(installed('paper'), paper)]);
    expect(await host.unloadMod('paper')).toBe(true);
    await host.reload();
    expect(colors()).toEqual(['Paper', undefined]);
    expect(host.active().map((mod) => mod.name)).toEqual([]);
  });

  test('a new session without the uninstalled plugin sets no colours', async () => {
    const { host, colors } = colorsSession();
    await host.load([]);
    expect(colors()).toEqual([]);
  });
});

const MOD_FILES = (folder: string) => ({
  [`${folder}/.claude-plugin/plugin.json`]: '{"name":"paper"}',
  [`${folder}/hooks/hooks.json`]: '{"modules":["./register.js"]}',
  [`${folder}/hooks/register.js`]: 'export function register(on) {}',
});

async function sessionPlugins(files: Record<string, string>): Promise<readonly string[]> {
  const { session: opened } = folderHostOf(files);
  const outcome = await opened.reloadSkills({ signal: signal() });
  return outcome.plugins.map((plugin) => plugin.id);
}

spec(
  'mods.interface.Remove mod colors from the Desktop app.You put the mod under ~/.claude/skills/',
  () => {
    describe('the mod loads from ~/.claude/skills/ and only from there', () => {
      test('under ~/.claude/skills/paper/ a session finds it as a plugin, paper@skills-dir', async () => {
        expect(await sessionPlugins(MOD_FILES(`${HOME}/.claude/skills/paper`))).toEqual([
          'paper@skills-dir',
        ]);
      });

      test('moved to ~/paper/, outside ~/.claude/skills/, the next session does not find it', async () => {
        expect(await sessionPlugins(MOD_FILES(`${HOME}/paper`))).toEqual([]);
      });
    });
  },
);
