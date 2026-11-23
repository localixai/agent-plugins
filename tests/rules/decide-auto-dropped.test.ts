/**
 * permission-modes.md (2.1.330) § "How the classifier evaluates actions": on entering auto mode the
 * broad allow rules that grant arbitrary code execution are dropped, now also a wrapper followed only
 * by a wildcard, a shell rule that starts with a wildcard and some wildcarded paths (Agent Plugins
 * Extended §13.9.4 item 3; which paths is a judgment, spec/judgments.md). The older kinds are in
 * decide-auto.test.ts.
 */
import { describe, expect, test } from 'bun:test';
import { allow, bash, classify, decision, rulesOf } from '../support/decide.ts';
import { spec } from '../support/spec.ts';

const AUTO = { autoAvailable: true };

/** `rule` written as an allow rule: left out in auto mode, back in effect in default mode. */
const dropped = (text: string, tool: string, input: unknown): void => {
  const rules = rulesOf(['allow', text]);
  expect(decision('auto', tool, input, { ...AUTO, rules })).toEqual(classify('mode-auto'));
  expect(decision('default', tool, input, { rules })).toEqual(allow('allow-rule'));
};

spec('permissions.mode.auto', () => {
  describe('on entering auto mode, broad allow rules are dropped', () => {
    test('2.1.330: a wrapper followed only by a wildcard', () => {
      dropped('Bash(timeout *)', 'Bash', { command: 'timeout 5 curl https://example.com' });
      dropped('Bash(nohup *)', 'Bash', { command: 'nohup ./serve' });
      dropped('Bash(xargs:*)', 'Bash', { command: 'xargs rm' });
    });

    test('2.1.330: a shell rule that starts with a wildcard', () => {
      dropped('Bash(* --help)', 'Bash', { command: 'curl --help' });
    });

    test('2.1.330: wildcarded paths such as Bash($HOME/bin/*) and PowerShell(C:\\tools\\x.exe *)', () => {
      dropped('Bash($HOME/bin/*)', 'Bash', { command: '$HOME/bin/deploy' });
      dropped('Bash(/usr/local/bin/tool *)', 'Bash', { command: '/usr/local/bin/tool run' });
      dropped('PowerShell(C:\\tools\\x.exe *)', 'PowerShell', { command: 'C:\\tools\\x.exe go' });
    });

    test('a wrapper with a fixed command, or a path without a wildcard, stays', () => {
      for (const [rule, command] of [
        ['Bash(timeout 5 npm test)', 'timeout 5 npm test'],
        ['Bash(/usr/local/bin/tool)', '/usr/local/bin/tool'],
        ['Bash(git status *)', 'git status --short'],
      ] as const) {
        const rules = rulesOf(['allow', rule]);
        expect(bash('auto', command, { ...AUTO, rules }), rule).toEqual(allow('allow-rule'));
      }
    });
  });
});
