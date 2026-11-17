/**
 * plugins/org.md § "Marketplaces hosted on claude.ai": the allowlist and blocklist match one by its
 * host; a `hostPattern` matching `claude.ai` on the allowlist admits the organization's and the
 * claude.ai default marketplaces, not one of a member's own uploads or one of unstated scope. No
 * requirement id covers the paragraph, so the tests stand outside `spec(...)` (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import { matchClaudeAiMarketplace } from '../../../src/settings/marketplaces/index.ts';
import {
  directory,
  file,
  git,
  github,
  hostPattern,
  managed,
  pathPattern,
  skillsDir,
  url,
} from '../../support/marketplace-sources.ts';

describe('marketplaces hosted on claude.ai', () => {
  test('with no list every claude.ai marketplace loads', () => {
    expect(managed({}).checkClaudeAi('claudeai-team', undefined)).toEqual({ loads: true });
  });

  test('an allowlist hostPattern for claude.ai admits the organization and default scopes only', () => {
    const gate = managed({ strictKnownMarketplaces: [hostPattern('^claude\\.ai$')] });
    expect(gate.checkClaudeAi('claudeai-team', 'organization')).toEqual({ loads: true });
    expect(gate.checkClaudeAi('claudeai-default', 'default')).toEqual({ loads: true });
    for (const scope of ['member', undefined] as const) {
      const verdict = gate.checkClaudeAi('claudeai-mine', scope);
      expect(verdict.loads).toBe(false);
    }
  });

  test('an allowlist without a claude.ai hostPattern admits none', () => {
    const gate = managed({
      strictKnownMarketplaces: [
        hostPattern('^github\\.com$'),
        github('acme/plugins'),
        url('https://claude.ai'),
      ],
    });
    expect(gate.checkClaudeAi('claudeai-team', 'organization')).toMatchObject({ loads: false });
    expect(managed({ strictKnownMarketplaces: [] }).checkClaudeAi('x', 'default').loads).toBe(
      false,
    );
  });

  test('a blocklist hostPattern for claude.ai blocks every scope', () => {
    const gate = managed({
      blockedMarketplaces: [hostPattern('claude\\.ai')],
      strictKnownMarketplaces: [hostPattern('claude\\.ai')],
    });
    expect(gate.checkClaudeAi('claudeai-team', 'organization')).toEqual({
      loads: false,
      message:
        'Marketplace "claudeai-team" is blocked by enterprise policy\nThis marketplace source is explicitly blocked by your administrator',
    });
    expect(
      managed({ blockedMarketplaces: [hostPattern('gitlab')] }).checkClaudeAi('a', undefined),
    ).toEqual({
      loads: true,
    });
  });

  test('the policy facts of a claude.ai marketplace: its host is claude.ai, and only a hostPattern names it', () => {
    for (const list of ['strict', 'blocked'] as const) {
      expect(matchClaudeAiMarketplace(list, hostPattern('^claude\\.ai$'))).toBe(true);
      expect(matchClaudeAiMarketplace(list, hostPattern('^github\\.com$'))).toBe(false);
      for (const entry of [
        url('https://claude.ai'),
        url('https://claude.ai/'),
        git('https://claude.ai/x.git'),
        github('claude/ai'),
        file('/claude.ai'),
        directory('/claude.ai'),
        pathPattern('.*'),
        skillsDir,
      ]) {
        expect(matchClaudeAiMarketplace(list, entry)).toBe(false);
      }
    }
  });
});
