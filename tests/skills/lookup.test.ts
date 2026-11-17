/**
 * skills.md § Names reserved for synced skills: "A plugin named `anthropic-skills`: it loads. When
 * one of its skills and a synced skill are both named `<name>`, `/anthropic-skills:<name>` runs the
 * synced skill." The lookup that serves the `/` prompt, the Skill call and a subagent's `skills`.
 */
import { expect, test } from 'bun:test';
import { skillLookup } from '../../src/skills/lookup.ts';
import { skillWith } from '../support/skills.ts';
import { spec } from '../support/spec.ts';

const PLUGIN_DEPLOY = skillWith('description: Plugin deploy', {
  origin: { kind: 'plugin', plugin: 'anthropic-skills' },
});
const SYNCED_DEPLOY = skillWith('description: Synced deploy', { origin: { kind: 'synced' } });

const row = (invocable: typeof PLUGIN_DEPLOY) => ({ invocable, names: [invocable.command] });

spec('skills.behavior.Choose where skills load.Reserved name anthropic-skills', () => {
  test('a plugin named anthropic-skills and a synced skill share a name: /anthropic-skills:<name> runs the synced skill', () => {
    expect(PLUGIN_DEPLOY.command).toBe('anthropic-skills:deploy');
    expect(SYNCED_DEPLOY.command).toBe('anthropic-skills:deploy');
    // Plugins come first in the catalog; the synced skill still takes the command.
    const lookup = skillLookup([row(PLUGIN_DEPLOY), row(SYNCED_DEPLOY)]);
    expect(lookup.find('anthropic-skills:deploy')?.invocable).toBe(SYNCED_DEPLOY);
  });

  test("the plugin's other skills keep their commands", () => {
    const review = skillWith('description: Review', {
      name: 'review',
      origin: { kind: 'plugin', plugin: 'anthropic-skills' },
    });
    const lookup = skillLookup([row(PLUGIN_DEPLOY), row(review), row(SYNCED_DEPLOY)]);
    expect(lookup.find('anthropic-skills:review')?.invocable).toBe(review);
  });
});
