/**
 * Spec §14.3.2 item 3 ("The cache expires 24 hours after the last successful fetch: a startup fetch
 * that fails before then applies the cache, with a warning ... that the cached policy is in effect;
 * one that fails after then starts without server-managed settings, the warning saying no remote
 * policy applies, unless `forceRemoteSettingsRefresh` makes the client exit") and item 4 (the fetch
 * goes through the proxy `HTTPS_PROXY` names, "from an endpoint-managed `env`, the launch environment
 * or user settings; `ALL_PROXY` alone does not serve it"); §14.1.2 (the cache "can apply at startup
 * until it expires"); errors.md, "Remote managed settings failed to load" (no cached settings to
 * apply: "none fetched yet on this machine, or the last fetch more than 24 hours ago").
 */
import { describe, expect, test } from 'bun:test';
import {
  SERVER_CACHE_LIFETIME_MS,
  serverFetchProxy,
  startupOnFailedFetch,
} from '../../../src/settings/managed/index.ts';
import { remoteManagedSettingsFailed } from '../../../src/format/host-errors.ts';
import { spec } from '../../support/spec.ts';

const DAY = 24 * 60 * 60 * 1000;
const FETCHED = 1_700_000_000_000;
const cache = { settings: { model: 'org' }, fetchedAt: FETCHED };

spec('policy.managed.Where each mechanism stores the policy.Server-managed', () => {
  test('the cache lives 24 hours after the last successful fetch', () => {
    expect(SERVER_CACHE_LIFETIME_MS).toBe(DAY);
  });

  test('a startup fetch that fails before the cache expires applies it, saying the cached policy is in effect', () => {
    const started = startupOnFailedFetch(cache, { now: FETCHED + DAY - 1 });
    expect(started).toEqual({ outcome: 'starts', settings: { model: 'org' }, policy: 'cached' });
    if (started.outcome === 'exits') throw new Error('expected a start');
    expect(remoteManagedSettingsFailed('network error', started.policy)).toBe(
      'Remote managed settings failed to load (network error): using cached policy',
    );
  });
});

spec('cli.errors.Remote managed settings failed to load.No cached settings to apply', () => {
  test('a fetch that fails 24 hours or more after the last success starts without server-managed settings', () => {
    for (const now of [FETCHED + DAY, FETCHED + 3 * DAY]) {
      expect(startupOnFailedFetch(cache, { now })).toEqual({ outcome: 'starts', policy: 'none' });
    }
  });

  test('none fetched yet on this machine: no remote policy applied', () => {
    expect(startupOnFailedFetch(undefined, { now: FETCHED })).toEqual({
      outcome: 'starts',
      policy: 'none',
    });
  });

  test('under forceRemoteSettingsRefresh a failed fetch exits instead, cache or not', () => {
    for (const held of [cache, undefined]) {
      expect(
        startupOnFailedFetch(held, { now: FETCHED + 1, forceRemoteSettingsRefresh: true }),
      ).toEqual({ outcome: 'exits' });
    }
  });
});

describe('the proxy of the fetch (spec §14.3.2 item 4)', () => {
  test('HTTPS_PROXY from an endpoint-managed env, the launch environment or user settings', () => {
    expect(serverFetchProxy({ endpointEnv: { HTTPS_PROXY: 'http://org:1' } })).toBe('http://org:1');
    expect(serverFetchProxy({ launchEnv: { HTTPS_PROXY: 'http://shell:1' } })).toBe(
      'http://shell:1',
    );
    expect(serverFetchProxy({ userEnv: { HTTPS_PROXY: 'http://user:1' } })).toBe('http://user:1');
  });

  test('ALL_PROXY alone does not serve it, and no proxy is none', () => {
    expect(serverFetchProxy({ launchEnv: { ALL_PROXY: 'http://all:1' } })).toBeUndefined();
    expect(serverFetchProxy({})).toBeUndefined();
  });

  test('the endpoint-managed value wins, then user settings over the launch environment (judgment)', () => {
    const all = {
      endpointEnv: { HTTPS_PROXY: 'http://org:1' },
      userEnv: { HTTPS_PROXY: 'http://user:1' },
      launchEnv: { HTTPS_PROXY: 'http://shell:1' },
    };
    expect(serverFetchProxy(all)).toBe('http://org:1');
    expect(serverFetchProxy({ ...all, endpointEnv: {} })).toBe('http://user:1');
  });

  test('an empty value is unset and lets the next source serve', () => {
    expect(
      serverFetchProxy({
        endpointEnv: { HTTPS_PROXY: '' },
        launchEnv: { HTTPS_PROXY: 'http://s:1' },
      }),
    ).toBe('http://s:1');
  });
});
