/**
 * Reloading a mod on save (Agent Plugins Extended §10.8.2 item 1, the settings table of §10.12.2,
 * §3 `CLAUDE_CODE_PLUGIN_DIR_WATCH`; Claude Code 2.1.330).
 */
import { expect, test } from 'bun:test';
import type { ModsPolicyInput, ReloadSource } from '../../../src/mods/load/index.ts';
import { policy, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

const reloads = (
  settings: ModsPolicyInput,
  source: ReloadSource,
  facts: { readonly isInteractiveTerminal: boolean; readonly takesMorePrompts: boolean },
) => session({}, { policy: settings }).host.reloadsOnSave(source, facts);

spec(
  ['mods.setting.CLAUDE_CODE_PLUGIN_DIR_WATCH', 'environment.host.CLAUDE_CODE_PLUGIN_DIR_WATCH'],
  () => {
    // §10.8.2 item 1 and the settings table (2.1.330): unset, skills-directory mods reload in any
    // session that takes more than one prompt, `--plugin-dir` mods only in an interactive terminal
    // session; `1` reloads `--plugin-dir` mods as skills-directory mods; `0` turns reloading off
    // everywhere. Read as an on/off variable (J3.16).
    const terminal = { isInteractiveTerminal: true, takesMorePrompts: true };
    const longHeadless = { isInteractiveTerminal: false, takesMorePrompts: true };
    const onePrompt = { isInteractiveTerminal: false, takesMorePrompts: false };

    test('unset: a skills-directory mod reloads in any session of more than one prompt, a --plugin-dir mod only in an interactive terminal', () => {
      expect(reloads(policy(), 'skills-dir', longHeadless)).toBe(true);
      expect(reloads(policy(), 'skills-dir', onePrompt)).toBe(false);
      expect(reloads(policy(), 'plugin-dir', terminal)).toBe(true);
      expect(reloads(policy(), 'plugin-dir', longHeadless)).toBe(false);
    });

    test('`1` makes --plugin-dir mods reload as skills-directory mods do; another value leaves the default', () => {
      for (const value of ['1', 'true', 'On']) {
        const on = policy({ env: { CLAUDE_CODE_PLUGIN_DIR_WATCH: value } });
        expect(reloads(on, 'plugin-dir', longHeadless)).toBe(true);
        expect(reloads(on, 'plugin-dir', onePrompt)).toBe(false);
      }
      for (const value of ['maybe', '']) {
        const other = policy({ env: { CLAUDE_CODE_PLUGIN_DIR_WATCH: value } });
        expect(reloads(other, 'plugin-dir', longHeadless)).toBe(false);
      }
    });

    test('`0` turns reloading off everywhere, skills-directory mods included', () => {
      for (const value of ['0', 'false', 'OFF']) {
        const off = policy({ env: { CLAUDE_CODE_PLUGIN_DIR_WATCH: value } });
        expect(reloads(off, 'plugin-dir', terminal)).toBe(false);
        expect(reloads(off, 'skills-dir', terminal)).toBe(false);
      }
    });
  },
);
