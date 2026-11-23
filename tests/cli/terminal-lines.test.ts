/**
 * Spec §17.2.1 (plugins/cli-reference.md, "claude plugin commands"; Judgment J17.33): a success line
 * quoted as `Successfully <verb> …` keeps `Successfully` only when stdout or stderr is not a
 * terminal; in a terminal it begins with the capitalised verb (`Installed plugin: <id> (scope:
 * <scope>)`, `Added marketplace: <name> (declared in <scope> settings)`). The `message` of a `--json`
 * result keeps `Successfully` either way. §17.2.1 carries no requirement id, so these tests run
 * outside `spec(...)`; `terminal` as a render option is the kit's (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import {
  commandFailed,
  commandSucceeded,
  marketplaceSuccess,
  renderCommand,
} from '../../src/cli/result.ts';
import { shellInstallLines } from '../../src/marketplace/install-commands.ts';

const ORACLE: readonly (readonly [string, string])[] = [
  ['Successfully installed plugin: p@mk (scope: user)', 'Installed plugin: p@mk (scope: user)'],
  ['Successfully enabled plugin: p (scope: user)', 'Enabled plugin: p (scope: user)'],
  ['Successfully disabled plugin: p (scope: user)', 'Disabled plugin: p (scope: user)'],
  ['Successfully updated marketplace: mk', 'Updated marketplace: mk'],
  ['Successfully updated 2 marketplaces', 'Updated 2 marketplaces'],
  ['Successfully uninstalled plugin: p (scope: user)', 'Uninstalled plugin: p (scope: user)'],
  ['Successfully removed marketplace: mk', 'Removed marketplace: mk'],
  [
    'Successfully added marketplace: mk (declared in user settings)',
    'Added marketplace: mk (declared in user settings)',
  ],
  [
    'Successfully updated plugin: p to version 2.0.0 (scope: user)',
    'Updated plugin: p to version 2.0.0 (scope: user)',
  ],
];

describe('J17.33: success lines in a terminal', () => {
  test('each quoted line drops Successfully and starts with the capitalised verb', () => {
    for (const [piped, terminal] of ORACLE) {
      const output = commandSucceeded('install', piped);
      expect(renderCommand(output, { json: false, terminal: true }).stdout).toEqual([terminal]);
    }
  });

  test('piped, or with no terminal fact, the line keeps Successfully', () => {
    for (const [piped] of ORACLE) {
      const output = commandSucceeded('install', piped);
      expect(renderCommand(output, { json: false, terminal: false }).stdout).toEqual([piped]);
      expect(renderCommand(output, { json: false }).stdout).toEqual([piped]);
    }
  });

  test('the --json message keeps Successfully in a terminal', () => {
    const output = marketplaceSuccess(
      'marketplace-remove',
      'mk',
      'Successfully removed marketplace: mk',
    );
    const line = renderCommand(output, { json: true, terminal: true }).stdout.at(-1) ?? '{}';
    expect(JSON.parse(line).message).toBe('Successfully removed marketplace: mk');
  });

  test('the lines printed before the result follow the same rule', () => {
    const output = commandSucceeded('install', 'Successfully installed plugin: p@mk (scope: user)');
    const rendered = renderCommand(output, {
      json: false,
      terminal: true,
      before: ['Successfully added marketplace: mk (declared in user settings)', 'Run: ./x'],
    });
    expect(rendered.stdout).toEqual([
      'Added marketplace: mk (declared in user settings)',
      'Run: ./x',
      'Installed plugin: p@mk (scope: user)',
    ]);
  });

  test('a failure line is printed as it is', () => {
    const output = commandFailed('install', 'Successfully nothing');
    expect(renderCommand(output, { json: false, terminal: true }).stderr).toEqual([
      'Successfully nothing',
    ]);
  });

  test("the shell's install lines in a terminal", () => {
    expect(
      shellInstallLines({
        pluginId: 'formatter@your-org',
        scope: 'project',
        addedMarketplace: 'your-org',
        terminal: true,
      }),
    ).toEqual([
      'Added marketplace: your-org (declared in user settings)',
      'Installed plugin: formatter@your-org (scope: project)',
    ]);
  });
});
