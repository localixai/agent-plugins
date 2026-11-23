/**
 * errors.md, "Remote managed settings failed to load" (spec §17.20 table, §14.3.2): the warning names
 * the cause and the policy the session runs on; with no cached settings to apply (none fetched yet,
 * or the last fetch more than 24 hours ago) the session runs without server-managed settings.
 */
import { expect, test } from 'bun:test';
import {
  REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD,
  REMOTE_MANAGED_SETTINGS_STILL_LOADING,
  remoteManagedSettingsFailed,
  remoteManagedSettingsLoaded,
} from '../../src/format/host-errors.ts';
import { startupOnFailedFetch } from '../../src/settings/managed/server-cache.ts';
import { spec } from '../support/spec.ts';

const E = 'cli.errors.';
const HOUR = 60 * 60 * 1000;

spec(
  [
    `${E}Remote managed settings failed to load`,
    `${E}Remote managed settings failed to load.No cached settings to apply`,
    `${E}Remote managed settings failed to load.Remote managed settings did not load`,
    `${E}Remote managed settings failed to load.Remote managed settings have now loaded`,
  ],
  () => {
    test('the warning names the cause and the policy the session runs on', () => {
      const cached = remoteManagedSettingsFailed('request timed out', 'cached');
      expect(cached).toStartWith('Remote managed settings failed to load (request timed out)');
      expect(cached).toEndWith('using cached policy');
      expect(remoteManagedSettingsFailed('network error', 'none')).toEndWith(
        'no remote policy applied',
      );
    });

    test('no cached settings to apply: none fetched yet, or the last fetch more than 24 hours ago', () => {
      const now = 100 * HOUR;
      const cache = (age: number) => ({ settings: {}, fetchedAt: now - age });
      const policyOf = (age: number | undefined) => {
        const start = startupOnFailedFetch(age === undefined ? undefined : cache(age), { now });
        return 'policy' in start ? start.policy : 'exits';
      };
      expect(policyOf(undefined)).toBe('none');
      expect(policyOf(25 * HOUR)).toBe('none');
      expect(policyOf(23 * HOUR)).toBe('cached');
      expect(
        remoteManagedSettingsFailed(
          'network error',
          policyOf(25 * HOUR) === 'none' ? 'none' : 'cached',
        ),
      ).toBe('Remote managed settings failed to load (network error): no remote policy applied');
    });

    test('the related state lines', () => {
      expect(REMOTE_MANAGED_SETTINGS_STILL_LOADING).toBe(
        'Remote managed settings are still loading',
      );
      expect(REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD).toBe('Remote managed settings did not load');
      expect(remoteManagedSettingsLoaded(true)).toStartWith(
        'Remote managed settings have now loaded',
      );
      expect(remoteManagedSettingsLoaded(false)).toEndWith('no remote policy applied');
    });
  },
);
