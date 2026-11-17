/**
 * permission-modes.md § How the classifier evaluates actions ("A shell command that carries
 * per-command allowed domains also routes to the classifier even when an allow rule matches, because
 * a rule approves the command, not its hosts"; "On entering auto mode, broad allow rules that grant
 * arbitrary code execution are dropped"), sandboxing.md § Per-command allowed domains in auto mode,
 * and settings-reference.md, `autoMode.classifyAllShell` ("interpreter or shell-wrapper prefixes").
 * Which shell wrappers count is a judgment (spec/judgments.md, `src/rules/decide/auto-dropped.ts`).
 */
import { describe, expect, test } from 'bun:test';
import { decide } from '../../src/rules/index.ts';
import { allow, ask, baseContext, classify, decision, rulesOf } from '../support/decide.ts';
import { spec } from '../support/spec.ts';

const AUTO = { autoAvailable: true };
const npmInstall = { toolName: 'Bash', input: { command: 'npm install' } };
const hosted = { ...npmInstall, allowedDomains: ['registry.npmjs.org'] };
const allowed = rulesOf(['allow', 'Bash(npm install)']);

spec('permissions.mode.auto', () => {
  describe('a command that carries per-command allowed domains', () => {
    test('goes to the classifier even when an allow rule matches', () => {
      expect(decide(hosted, baseContext('auto', { ...AUTO, rules: allowed }))).toEqual(
        classify('mode-auto'),
      );
    });

    test('without hosts the same allow rule approves it', () => {
      expect(decide(npmInstall, baseContext('auto', { ...AUTO, rules: allowed }))).toEqual(
        allow('allow-rule'),
      );
    });

    test('an ask rule still prompts for it', () => {
      const rules = rulesOf(['ask', 'Bash(npm install)']);
      expect(decide(hosted, baseContext('auto', { ...AUTO, rules }))).toEqual(ask('ask-rule'));
    });

    test('outside auto mode the allow rule decides', () => {
      expect(decide(hosted, baseContext('default', { rules: allowed }))).toEqual(
        allow('allow-rule'),
      );
    });
  });
});

describe('shell-wrapper allow rules are suspended in auto mode (judgment J13.24)', () => {
  for (const [rule, command] of [
    ['Bash(bash *)', 'bash deploy.sh'],
    ['Bash(sh -c *)', 'sh -c "rm -rf build"'],
    ['Bash(zsh:*)', 'zsh run.zsh'],
  ] as const) {
    test(`${rule} is dropped in auto and back in default`, () => {
      const rules = rulesOf(['allow', rule]);
      expect(decision('auto', 'Bash', { command }, { ...AUTO, rules })).toEqual(
        classify('mode-auto'),
      );
      expect(decision('default', 'Bash', { command }, { rules })).toEqual(allow('allow-rule'));
    });
  }

  test('a narrow wrapper rule without a wildcard stays', () => {
    const rules = rulesOf(['allow', 'Bash(bash deploy.sh)']);
    expect(decision('auto', 'Bash', { command: 'bash deploy.sh' }, { ...AUTO, rules })).toEqual(
      allow('allow-rule'),
    );
  });
});
