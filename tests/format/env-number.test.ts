/**
 * env-vars.md, "Variables" (Agent Plugins Extended §3.6.1 item 2, J3.17): a numeric variable takes
 * plain digits, scientific notation (`2e3` is 2000) and digit separators (`64_000` is 64000), unless
 * its row says plain digits only; a value that cannot be read leaves the default. The readers are a
 * judgment of the spec, tested outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import {
  plainPositiveWholeEnv,
  positiveWholeEnv,
  readNumberEnv,
  wholeEnv,
} from '../../src/format/env-number.ts';
import { mcpDescriptionLimits, mcpTimeouts } from '../../src/entry/format.ts';
import { pluginGitTimeoutMs, syncPluginInstall } from '../../src/settings/host-switches.ts';
import { fakeEnvironment } from '../support/ports.ts';

describe('a numeric variable', () => {
  test('reads plain digits, scientific notation and digit separators', () => {
    expect(readNumberEnv('2000')).toBe(2000);
    expect(readNumberEnv('2e3')).toBe(2000);
    expect(readNumberEnv('64_000')).toBe(64_000);
    expect(readNumberEnv('1.5')).toBe(1.5);
  });

  test('anything else cannot be read', () => {
    for (const text of [
      undefined,
      '',
      'soon',
      '-5',
      ' 7',
      '0x10',
      '_1',
      '1_',
      '1__0',
      'Infinity',
    ]) {
      expect(readNumberEnv(text)).toBeUndefined();
    }
  });

  test('the whole and positive readers keep their bounds', () => {
    expect(wholeEnv('0')).toBe(0);
    expect(wholeEnv('2.5')).toBeUndefined();
    expect(positiveWholeEnv('0')).toBeUndefined();
    expect(positiveWholeEnv('2.5e3')).toBe(2500);
  });

  test('a plain-digits variable reads digits only', () => {
    expect(plainPositiveWholeEnv('2e3')).toBeUndefined();
    expect(plainPositiveWholeEnv('64_000')).toBeUndefined();
    expect(plainPositiveWholeEnv('4096')).toBe(4096);
  });
});

describe('the variables that read it', () => {
  const lookup =
    (record: Record<string, string>) =>
    (name: string): string | undefined =>
      record[name];

  test('MCP_TIMEOUT 2e3 and the plugin timeouts 64_000 are read; the description limit stays plain digits', () => {
    expect(mcpTimeouts(lookup({ MCP_TIMEOUT: '2e3' }), { transport: 'stdio' }).startupMs).toBe(
      2000,
    );
    expect(
      pluginGitTimeoutMs(fakeEnvironment({ CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS: '64_000' })),
    ).toBe(64_000);
    const sync = fakeEnvironment({
      CLAUDE_CODE_SYNC_PLUGIN_INSTALL: '1',
      CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS: '2e3',
    });
    expect(syncPluginInstall(sync, { interactive: false })).toEqual({
      wait: true,
      timeoutMs: 2000,
    });
    expect(
      mcpDescriptionLimits(lookup({ CLAUDE_CODE_MAX_MCP_DESCRIPTION_LENGTH: '2e3' })).upfront,
    ).toBe(mcpDescriptionLimits(lookup({})).upfront);
  });
});
