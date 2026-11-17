/**
 * errors.md § Marketplace entry path does not stay inside the marketplace directory: `claude plugin
 * install` reports `Cannot install <id>: its marketplace entry path does not stay inside the
 * marketplace directory (…)`; plugins/troubleshooting.md § Plugins with relative paths fail in
 * URL-based marketplaces. The parenthesis is the kit's own (spec J17.30).
 */
import { expect, test } from 'bun:test';
import { planInstallStart } from '../../src/marketplace/install-plan.ts';
import { spec } from '../support/spec.ts';

const PREFIX =
  'Cannot install formatter@acme: its marketplace entry path does not stay inside the marketplace directory (';

spec('cli.errors.Marketplace entry path does not stay inside the marketplace directory', () => {
  test('a relative entry of a marketplace added by URL is refused at install, naming the plugin', () => {
    const started = planInstallStart({
      source: { kind: 'relative', path: './plugins/formatter', bare: false },
      fetch: {
        platform: 'posix',
        pluginId: 'formatter@acme',
        marketplaceOrigin: { kind: 'url', url: 'https://example.com/marketplace.json' },
      },
    });
    expect(started.fetch).toBeUndefined();
    expect(started.issues).toHaveLength(1);
    expect(started.issues[0]?.severity).toBe('error');
    expect(started.issues[0]?.message.startsWith(PREFIX)).toBe(true);
    expect(started.issues[0]?.message.endsWith(')')).toBe(true);
  });
  test('a backslash in the entry path is refused at install on macOS and Linux, not on Windows', () => {
    const source = { kind: 'relative', path: './plugins\\formatter', bare: false } as const;
    const posix = planInstallStart({
      source,
      fetch: { platform: 'posix', pluginId: 'formatter@acme' },
    });
    expect(posix.fetch).toBeUndefined();
    expect(posix.issues[0]?.message.startsWith(PREFIX)).toBe(true);
    const windows = planInstallStart({ source, fetch: { platform: 'windows' } });
    expect(windows.issues.filter((found) => found.message.startsWith('Cannot install'))).toEqual(
      [],
    );
  });
});
