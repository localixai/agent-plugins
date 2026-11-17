/**
 * Agent Plugins Extended §16.6.2 item 1 (plugins/troubleshooting.md, "installed_plugins.json holds a
 * record under ... that this version of Claude Code cannot read"): while such a record exists,
 * `claude plugin install`, `uninstall` and `update` refuse with `Plugin "<name>" was not
 * installed:` (`uninstalled`, `updated`) and the same text, and `--json` carries
 * `failureCode: "install_records_unreadable"`.
 */
import { expect, test } from 'bun:test';
import { renderCommand } from '../../src/cli/result.ts';
import { startUninstall } from '../../src/cli/uninstall-plan.ts';
import { unreadableRecordsMessage } from '../../src/marketplace/install-messages.ts';
import { lastJson, run } from '../support/install.ts';
import { ID, world } from '../support/uninstall.ts';
import { install, outputOf, update } from '../support/update.ts';
import { spec } from '../support/spec.ts';

const RECORD =
  'install.troubleshooting.installed_plugins.json holds a record under "<id>" that this version of Claude Code cannot read';

const TEXT = unreadableRecordsMessage(['old@m']);

spec([RECORD, `${RECORD}.--json on any of those three commands`], () => {
  test('install', async () => {
    const found = await run({ found: { unreadableRecords: TEXT } });
    expect(lastJson(found)).toMatchObject({
      command: 'install',
      outcome: 'failed',
      failureCode: 'install_records_unreadable',
      message: `Plugin "formatter@m" was not installed: ${TEXT}`,
    });
    expect('effects' in found).toBe(false);
  });

  test('uninstall', () => {
    const started = startUninstall(
      { plugin: ID, scope: 'project' },
      { graph: world([ID, { scope: 'project' }]), tty: true, unreadableRecords: TEXT },
    );
    if (started.kind !== 'done') throw new Error('the uninstall went ahead');
    const line = JSON.parse(renderCommand(started.output, { json: true }).stdout.at(-1) ?? '{}');
    expect(line).toMatchObject({
      command: 'uninstall',
      failureCode: 'install_records_unreadable',
      message: `Plugin "${ID}" was not uninstalled: ${TEXT}`,
    });
  });

  test('update', async () => {
    const found = await update({
      installs: [install('formatter@m', { installed: '1.0.0', offered: '2.0.0' })],
      facts: { unreadableRecords: TEXT },
    });
    const output = outputOf(found);
    if (output?.kind !== 'result') throw new Error('the update did not fail with a result');
    expect(output.result).toMatchObject({
      command: 'update',
      outcome: 'failed',
      failureCode: 'install_records_unreadable',
      message: `Plugin "formatter@m" was not updated: ${TEXT}`,
    });
  });
});
