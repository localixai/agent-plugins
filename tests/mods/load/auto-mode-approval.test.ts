/**
 * Agent Plugins Extended §14.9.2 item 4 (plugins/mods/admin.md § Know what happens by default, "When
 * a user's mod approves Claude's call"; permissions.md of 2.1.330, "Precedence among rules, hooks,
 * and mods"; §13.11 item 2): skipping auto mode's classifier is the setup without the guard only.
 */
import { expect, test } from 'bun:test';
import { modApprovalSkipsClassifier } from '../../../src/mods/load/index.ts';
import { spec } from '../../support/spec.ts';

spec("policy.mods.Know what happens by default.When a user's mod approves Claude's call", () => {
  test('without the guard, a call a user`s mod approves in auto mode runs without the classifier`s check', () => {
    expect(modApprovalSkipsClassifier({ guardLoaded: false })).toBe(true);
  });

  test('where the guard loads, the classifier still checks the call, with or without a prompt tool or canUseTool callback', () => {
    expect(modApprovalSkipsClassifier({ guardLoaded: true })).toBe(false);
  });
});
