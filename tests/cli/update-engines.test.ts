/**
 * Spec §2.4 (`engines`; changelog 2.1.323): when the newer version's `engines["claude-code"]` range
 * excludes the running client, `claude plugin update` does not move to it; the installed version
 * stays. The host offers "the newest version whose range fits", so a refused offer is no update.
 */
import { expect, test } from 'bun:test';
import { install, shown, update } from '../support/update.ts';
import { spec } from '../support/spec.ts';

const offered = (range: string, localFolder = false) =>
  install(
    'formatter@m',
    { installed: '1.0.0', offered: '1.1.0' },
    { engines: { engines: { 'claude-code': range }, clientVersion: '2.1.330', localFolder } },
  );

spec('manifest.field.engines', () => {
  test('a newer version outside the range is not updated to', async () => {
    const found = await update({ installs: [offered('>=2.1.400')] });
    expect('update' in found).toBe(false);
    expect(shown(found).exitCode).toBe(0);
    expect(shown(found).stdout.at(-1)).toBe('formatter is already at the latest version (1.0.0).');
  });

  test('a newer version inside the range, or read from a local folder, updates', async () => {
    for (const one of [offered('>=2.1.0'), offered('>=9.0.0', true)]) {
      const found = await update({ installs: [one] });
      expect('update' in found && found.update.to).toBe('1.1.0');
    }
  });
});
