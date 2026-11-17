/**
 * permission-modes.md § Eliminate prompts with auto mode, `disableAutoMode` ("A session already
 * running in auto mode leaves it when the setting reaches that session from an admin-deployed
 * source, and shows `auto mode disabled by settings`"); settings-reference.md, `disableAutoMode`.
 * That the session lands in Manual is a judgment (spec/judgments.md), asserted outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { AUTO_MODE_DISABLED_NOTICE } from '../../../src/settings/index.ts';
import { permissionsOf } from '../../support/permissions-policy.ts';
import { spec } from '../../support/spec.ts';

const managed = permissionsOf([['managed', { disableAutoMode: 'disable' }]]);
const nested = permissionsOf([['managed', { permissions: { disableAutoMode: 'disable' } }]]);
const user = permissionsOf([['user', { disableAutoMode: 'disable' }]]);

spec('permissions.setting.disableAutoMode', () => {
  test('a running auto session leaves auto when the value arrives from a managed source, with the notice', () => {
    expect(managed.runningMode('auto').notice).toBe('auto mode disabled by settings');
    expect(nested.runningMode('auto').notice).toBe(AUTO_MODE_DISABLED_NOTICE);
    expect(managed.runningMode('auto').mode).not.toBe('auto');
  });

  test('from a user file it does not move a running session', () => {
    expect(user.runningMode('auto')).toEqual({ mode: 'auto' });
  });

  test('a session in another mode is not moved', () => {
    expect(managed.runningMode('acceptEdits')).toEqual({ mode: 'acceptEdits' });
  });
});

describe('the mode it leaves for (judgment)', () => {
  test('is Manual, as a start in auto under the setting is', () => {
    expect(managed.runningMode('auto').mode).toBe('default');
  });
});
