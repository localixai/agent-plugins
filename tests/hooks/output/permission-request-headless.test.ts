/**
 * Agent Plugins Extended §7.5.12 `PermissionRequest` ("When", 2.1.330): in a `-p` run outside
 * `dontAsk` mode with neither a `--permission-prompt-tool` nor a `canUseTool` callback nobody can
 * answer a prompt, so a call that would prompt is denied unless something approves it, such as a
 * `PermissionRequest` hook; the hooks still run there. In `dontAsk` the denial is the mode's and no
 * hook lifts it (§13.9.5). Asserted through `decide` and `permissionRequestDecision`.
 */
import { describe, expect, test } from 'bun:test';
import { permissionRequestDecision } from '../../../src/index.ts';
import { decide } from '../../../src/rules/index.ts';
import { deny } from '../../support/decide.ts';
import { sessionOf } from '../../support/permission-session.ts';
import { headless, permissionsOf } from '../../support/permissions-policy.ts';
import { call } from '../../support/rule-context.ts';
import { spec } from '../../support/spec.ts';

const npmTest = call('Bash', { command: 'npm test' });
const answer = (behavior: 'allow' | 'deny') => ({ effects: { permissionRequest: { behavior } } });
const decided = (permissionMode: 'default' | 'dontAsk', callback = false) => {
  const { context } = sessionOf(
    permissionsOf([]),
    headless({ permissionMode }),
    callback ? { permissionCallback: true } : {},
  );
  return decide(npmTest, context);
};

spec('hooks.event.PermissionRequest', () => {
  describe('a -p run with nothing to answer a prompt', () => {
    test('the would-be prompt is denied, and the hook that allows it runs the call', () => {
      const denied = decided('default');
      expect(denied).toEqual(deny('prompt-unavailable'));
      expect(permissionRequestDecision(denied, answer('allow'))).toEqual({
        outcome: 'allow',
        reason: 'hook',
      });
    });

    test('no answer leaves the denial; a deny answer is the hook’s', () => {
      const denied = decided('default');
      expect(permissionRequestDecision(denied, { effects: {} })).toEqual(denied);
      expect(permissionRequestDecision(denied, answer('deny'))).toEqual(deny('hook'));
    });

    test('in dontAsk the denial is the mode’s: the hook does not lift it', () => {
      const denied = decided('dontAsk');
      expect(denied.outcome).toBe('deny');
      expect(permissionRequestDecision(denied, answer('allow'))).toEqual(denied);
    });

    test('with a canUseTool callback the call is a prompt the hooks answer alongside it', () => {
      const asked = decided('default', true);
      expect(asked).toEqual({ outcome: 'ask', reason: 'default-prompt' });
      expect(permissionRequestDecision(asked, answer('allow')).outcome).toBe('allow');
    });
  });
});
