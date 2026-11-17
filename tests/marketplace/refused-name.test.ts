/**
 * errors.md § Claude Code refuses the marketplace name: "Claude Code checks the name every time it
 * reads the marketplace's catalog"; a registered marketplace whose name imitates an official one
 * stops loading with its plugins, each reported with a message that begins `Claude Code refuses the
 * marketplace name "<name>"`, the marketplace's own error is `Claude Code refuses this marketplace's
 * name: it looks like one of Anthropic's own`, and for a marketplace declared inline with
 * `"source": "settings"` the message lists the steps that keep it (v2.1.292).
 */
import { describe, expect, test } from 'bun:test';
import type { MarketplaceSource } from '../../src/format/marketplace/types.ts';
import { refusedMarketplaceName } from '../../src/marketplace/install-lifecycle-index.ts';
import { spec } from '../support/spec.ts';

const GITHUB: MarketplaceSource = {
  kind: 'github',
  repo: 'someone/plugins',
  path: '.claude-plugin/marketplace.json',
};
const SETTINGS: MarketplaceSource = { kind: 'settings', name: 'anthropic-plugins-v2', plugins: [] };

spec('cli.errors.Claude Code refuses the marketplace name', () => {
  test('an imitating registered name stops loading: the plugin and marketplace messages', () => {
    const refused = refusedMarketplaceName('anthropic-plugins-v2', GITHUB);
    expect(refused?.marketplace).toBe(
      "Claude Code refuses this marketplace's name: it looks like one of Anthropic's own",
    );
    expect(
      refused?.plugin.startsWith('Claude Code refuses the marketplace name "anthropic-plugins-v2"'),
    ).toBe(true);
    expect(refused?.steps).toBeUndefined();
  });

  test('a name that does not impersonate loads', () => {
    expect(refusedMarketplaceName('acme-tools', GITHUB)).toBeUndefined();
    expect(refusedMarketplaceName('claude-plugins-official', GITHUB)).toBeUndefined();
  });

  test('a marketplace declared inline in settings gets the steps that keep it', () => {
    const steps = refusedMarketplaceName('anthropic-plugins-v2', SETTINGS)?.steps ?? [];
    expect(steps).toHaveLength(3);
    expect(steps[1]).toContain('claude plugin marketplace remove anthropic-plugins-v2');
  });
});

describe('judgment: the advice after the quoted prefix (spec/judgments.md)', () => {
  test('the plugin message says to remove the marketplace and what that deletes', () => {
    expect(refusedMarketplaceName('official-claude-plugins', GITHUB)?.plugin).toBe(
      'Claude Code refuses the marketplace name "official-claude-plugins": it looks like one of Anthropic\'s own. Remove it with claude plugin marketplace remove official-claude-plugins; this also uninstalls its plugins and deletes their data, options and secrets.',
    );
  });
});
