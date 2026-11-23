/**
 * Spec §16.24.1 item 4 ("When a dependency installs automatically": updating the dependent with
 * `claude plugin update` installs a missing declared dependency under the limits of an install,
 * except while `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set), §17.7 (the dependencies left unmet
 * are listed after `still unresolved:`) and §16.30 (`… still unresolved: <names>`, then
 * `Dependencies are not installed while non-essential traffic is turned off.`). The opening of the
 * `still unresolved` line and the dependency list of the success line are the kit's
 * (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import type { GraphFacts } from '../../src/marketplace/dependency-graph.ts';
import { facts, installed, plugin } from '../support/dependency-facts.ts';
import { install, shown, update } from '../support/update.ts';
import { spec } from '../support/spec.ts';

const newer = install('formatter@m', { installed: '1.0.0', offered: '1.1.0' });
const current = install('formatter@m', { installed: '1.1.0', offered: '1.1.0' });

/** The catalog's formatter declares `lib`; only formatter is installed. */
const graph: GraphFacts = facts(
  [plugin('formatter', { manifest: ['lib'] }), plugin('lib')],
  [installed('formatter@m')],
);

const NO_TRAFFIC = { CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '1' };

spec('dependencies.guide.When a dependency installs automatically', () => {
  test('an update to a newer version installs the dependency the plugin is missing', async () => {
    const found = await update({ installs: [newer], facts: { graph } });
    expect('dependencies' in found && found.dependencies?.map((step) => step.id)).toEqual([
      'lib@m',
    ]);
    expect(shown(found).stdout.at(-1)).toBe(
      'Successfully updated plugin: formatter to version 1.1.0 (scope: user) (+ 1 dependencies: lib@m)',
    );
  });

  test('an update that finds nothing newer still installs it', async () => {
    const found = await update({ installs: [current], facts: { graph } });
    expect('dependencies' in found && found.dependencies?.map((step) => step.id)).toEqual([
      'lib@m',
    ]);
    expect(shown(found).exitCode).toBe(0);
  });

  test('while non-essential traffic is turned off the update installs none', async () => {
    const found = await update({ installs: [newer], facts: { graph }, env: NO_TRAFFIC });
    expect('dependencies' in found ? found.dependencies : undefined).toBeUndefined();
    expect('update' in found && found.update?.to).toBe('1.1.0');
  });

  test('nothing is installed when every declared dependency is met', async () => {
    const met = facts(
      [plugin('formatter', { manifest: ['lib'] }), plugin('lib')],
      [installed('formatter@m'), installed('lib@m')],
    );
    const found = await update({ installs: [newer], facts: { graph: met } });
    expect('dependencies' in found ? found.dependencies : undefined).toBeUndefined();
    expect(shown(found).stdout.at(-1)).toBe(
      'Successfully updated plugin: formatter to version 1.1.0 (scope: user)',
    );
  });
});

spec('install.troubleshooting.Dependency errors.... still unresolved: <names>', () => {
  test('with traffic off the unmet dependencies are named, then why none was installed', async () => {
    const found = await update({ installs: [newer], facts: { graph }, env: NO_TRAFFIC });
    expect(shown(found).stdout.at(-1)).toBe(
      [
        'Successfully updated plugin: formatter to version 1.1.0 (scope: user)',
        'Dependencies still unresolved: lib@m',
        'Dependencies are not installed while non-essential traffic is turned off.',
      ].join('\n'),
    );
  });

  test('a dependency the update cannot install is still unresolved; the update itself succeeds', async () => {
    const unlisted = facts(
      [plugin('formatter', { manifest: ['ghost'] })],
      [installed('formatter@m')],
    );
    const found = await update({ installs: [newer], facts: { graph: unlisted } });
    const out = shown(found);
    expect(out.exitCode).toBe(0);
    expect(out.stdout.at(-1)).toContain('\nDependencies still unresolved: ghost@m');
    expect(out.stdout.at(-1)).not.toContain('non-essential traffic');
  });
});

describe('a catalog plugin whose plugin.json is unread', () => {
  test('is asked for, as an install asks', async () => {
    const unread = facts(
      [plugin('formatter', { manifest: ['lib'] }), plugin('lib')],
      [installed('formatter@m')],
    );
    const lib = unread.catalog['lib@m'];
    if (lib === undefined) throw new Error('no lib');
    const needing: GraphFacts = {
      ...unread,
      catalog: { ...unread.catalog, 'lib@m': { ...lib, manifestUnread: true } },
    };
    const found = await update({ installs: [newer], facts: { graph: needing } });
    expect('needs' in found && found.needs).toEqual(['lib@m']);
  });
});
