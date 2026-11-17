/**
 * settings-reference.md § Allow only the official marketplace ("registers the marketplace
 * automatically the first time you start an interactive terminal session"; what registration
 * misses: non-interactive runs, VS-Code-only machines, a recorded blocked attempt); env-vars.md,
 * `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL`. Spec 1.1.0 §14 "The official marketplace
 * under an allowlist" item 1.
 */
import { expect, test } from 'bun:test';
import {
  officialMarketplaceRegistration,
  type OfficialRegistrationFacts,
} from '../../../src/settings/marketplaces/index.ts';
import { managed, official } from '../../support/marketplace-sources.ts';
import { spec } from '../../support/spec.ts';

const onlyOfficial = managed({ strictKnownMarketplaces: [official] });
const fresh: OfficialRegistrationFacts = {
  launch: 'terminal',
  registered: false,
  skipRecorded: false,
  autoinstallEnabled: true,
};

spec('policy.reference.Allow only the official marketplace', () => {
  test('a fresh machine registers it at the first interactive terminal session', () => {
    expect(officialMarketplaceRegistration(onlyOfficial, fresh)).toEqual({ action: 'register' });
  });

  test('an already-registered official marketplace is kept: nothing to do', () => {
    expect(officialMarketplaceRegistration(onlyOfficial, { ...fresh, registered: true })).toEqual({
      action: 'none',
    });
  });

  test('not in a -p run, a terminal attached to a cloud session, or the VS Code extension', () => {
    for (const launch of ['print', 'cloud-terminal', 'vs-code'] as const) {
      expect([launch, officialMarketplaceRegistration(onlyOfficial, { ...fresh, launch })]).toEqual(
        [launch, { action: 'none' }],
      );
    }
  });

  test('a policy that blocks it, the empty allowlist included, records the attempt', () => {
    const lockdown = managed({ strictKnownMarketplaces: [] });
    expect(officialMarketplaceRegistration(lockdown, fresh)).toEqual({ action: 'record-skip' });
  });

  test('after a recorded blocked attempt it does not retry when the policy changes', () => {
    expect(officialMarketplaceRegistration(onlyOfficial, { ...fresh, skipRecorded: true })).toEqual(
      { action: 'none' },
    );
  });

  test('CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL skips the registration permanently', () => {
    expect(
      officialMarketplaceRegistration(onlyOfficial, { ...fresh, autoinstallEnabled: false }),
    ).toEqual({ action: 'record-skip' });
  });
});
