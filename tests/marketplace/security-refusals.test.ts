/** plugins/security.md § Untrusted marketplace sources and failed integrity checks. */
import { expect, test } from 'bun:test';
import { loadNameIssues } from '../../src/format/marketplace/marketplace-names.ts';
import type { MarketplaceSource } from '../../src/format/marketplace/types.ts';
import { untrustedSourceMessage } from '../../src/marketplace/ignored-marketplace.ts';
import { DIGEST, fetchWith, ok, refusal } from '../support/marketplace-archive.ts';
import { spec } from '../support/spec.ts';

spec('trust.security.Untrusted marketplace sources and failed integrity checks', () => {
  test('official and community names reject a non-Anthropic repository', () => {
    for (const name of ['claude-plugins-official', 'claude-community']) {
      const issues = loadNameIssues(name, {
        origin: { kind: 'github', repo: 'untrusted/plugins', path: 'm.json' },
      });
      expect(issues).toContainEqual(expect.objectContaining({ severity: 'error' }));
      expect(
        loadNameIssues(name, {
          origin: { kind: 'github', repo: 'anthropics/plugins', path: 'm.json' },
        }),
      ).toEqual([]);
    }
  });
  test('the untrusted-source message takes the middle sentence of its source kind', () => {
    const tail = 'To fix it, remove the marketplace and re-add it from the official source.';
    const github: MarketplaceSource = { kind: 'github', repo: 'acme/m', path: 'm.json' };
    const url: MarketplaceSource = { kind: 'url', url: 'https://example.com/m.json' };
    expect(untrustedSourceMessage('claude-community', github)).toContain(
      "Only repositories from 'github.com/anthropics/' can use this name. " + tail,
    );
    expect(untrustedSourceMessage('claude-community', url)).toContain(
      "can only be used with GitHub sources from the 'anthropics' organization. " + tail,
    );
  });
  test('a pinned archive with a different digest never reaches extraction', async () => {
    const run = fetchWith([ok], { plan: { sha256: DIGEST }, digest: '0'.repeat(64) });
    const failed = await refusal(run);
    expect(failed.message).toContain('Plugin archive integrity check failed');
    expect(run.archive.calls).toBe(0);
  });
});
