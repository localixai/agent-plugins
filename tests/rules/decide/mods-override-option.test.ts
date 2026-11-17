/**
 * plugins/mods/admin.md § Set options on the built-in guard, `allowModsToOverrideDenyRules`, as
 * Agent Plugins Extended §14.9.6 and Judgment J14.14 read "most of the outcomes that stay in place
 * by default": under policy the option lets a mod's approval past the deny and ask rules, the read
 * block and a hook outside managed settings, never a managed hook's decision or the permission
 * callback. A judgment, so outside `spec(...)`; asserted through `decide()`.
 */
import { describe, expect, test } from 'bun:test';
import { allow, ask, bash, deny, rulesOf } from '../../support/decide.ts';

const option = { approved: true, policyEnforced: true, overridesDenyRules: true };

describe('allowModsToOverrideDenyRules under policy (Agent Plugins Extended J14.14)', () => {
  test('lifts a deny rule, an ask rule and the read block', () => {
    expect(
      bash('default', 'curl x.io', { rules: rulesOf(['deny', 'Bash(curl *)']), mod: option }),
    ).toEqual(allow('mod'));
    expect(
      bash('default', 'git push', { rules: rulesOf(['ask', 'Bash(git push *)']), mod: option }),
    ).toEqual(allow('mod'));
    const block = { blockReadsOutsideWorkingDirectories: true, mod: option };
    expect(bash('default', 'cat /etc/passwd', block)).toEqual(allow('mod'));
  });

  test('lifts the block or prompt of a hook outside managed settings', () => {
    expect(bash('default', 'npm test', { hook: { blocked: true }, mod: option })).toEqual(
      allow('mod'),
    );
    expect(bash('default', 'npm test', { hook: { decision: 'ask' }, mod: option })).toEqual(
      allow('mod'),
    );
  });

  test("keeps a managed hook's block or prompt, with the option or without policy", () => {
    const managedBlock = { blocked: true, managed: true };
    expect(bash('default', 'npm test', { hook: managedBlock, mod: option })).toEqual(deny('hook'));
    expect(
      bash('default', 'npm test', { hook: { decision: 'ask', managed: true }, mod: option }),
    ).toEqual(ask('hook'));
    const free = { approved: true, policyEnforced: false };
    expect(bash('default', 'npm test', { hook: managedBlock, mod: free })).toEqual(deny('hook'));
  });

  test('keeps the permission callback: the call is still put to it', () => {
    expect(bash('default', 'npm test', { permissionCallback: true, mod: option })).toEqual(
      ask('default-prompt'),
    );
  });

  test('without the option, policy keeps every one of them', () => {
    const enforced = { approved: true, policyEnforced: true };
    expect(bash('default', 'npm test', { hook: { blocked: true }, mod: enforced })).toEqual(
      deny('hook'),
    );
  });
});
