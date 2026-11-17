/**
 * errors.md § Model switch was blocked by a PreModelSwitch hook (Agent Plugins Extended §7.5.33):
 * the report line and the client's own reasons. The docs section is no requirement of
 * `spec/requirements.json`, so its texts are tested outside `spec(...)`; the timeout reason a
 * hook's block carries is asserted under `spec(...)` in on-failure.test.ts.
 */
import { describe, expect, test } from 'bun:test';
import {
  cannotAskReason,
  managedHooksUncheckedReason,
  MODEL_SWITCH_REASONS,
  modelSwitchBlocked,
} from '../../../src/hooks/verdict/index.ts';

describe('a blocked model switch (errors.md)', () => {
  test('the report names the model for a switch the user made, and not for an SDK request', () => {
    expect(modelSwitchBlocked('retired', 'Opus 4.6')).toBe(
      'Model switch to Opus 4.6 was blocked by a PreModelSwitch hook: retired',
    );
    expect(modelSwitchBlocked('retired')).toBe(
      'Model switch blocked by a PreModelSwitch hook: retired',
    );
  });

  test('the client gives its own reasons, word for word', () => {
    expect(cannotAskReason()).toBe('confirmation required, and this session cannot ask');
    expect(cannotAskReason(true)).toBe(
      'confirmation required, and this session cannot ask (run /model interactively to confirm)',
    );
    expect(MODEL_SWITCH_REASONS.failed).toBe('a PreModelSwitch hook failed before answering');
    expect(MODEL_SWITCH_REASONS.cancelled).toBe(
      'PreModelSwitch hooks were cancelled (the control stream closed) before answering',
    );
    expect(managedHooksUncheckedReason('plugin acme failed to load')).toBe(
      'plugin acme failed to load, so organization-managed PreModelSwitch hooks could not be checked',
    );
  });
});
