/**
 * Agent Plugins Extended §7.5.25 `ConfigChange` ("When", 2.1.330): "Rules and settings the user
 * saves with **Yes, and don't ask again**, `/permissions` or `/model` take effect without the hooks
 * in the session they were saved from; another open session that loads the same file runs its
 * hooks" (`SettingsEdit.savedHere`).
 */
import { expect, test } from 'bun:test';
import { editEffect } from '../../../src/settings/general/index.ts';
import { spec } from '../../support/spec.ts';

spec('hooks.event.ConfigChange', () => {
  test('the session that saved the rule applies it without the ConfigChange hook', () => {
    for (const scope of ['user', 'project', 'local'] as const) {
      expect(
        editEffect({ scope, arrival: 'file', key: 'permissions.allow', savedHere: true }),
      ).toEqual({ applies: 'now', configChangeHook: false });
    }
  });

  test('another open session that loads the same file runs its hooks', () => {
    expect(
      editEffect({ scope: 'local', arrival: 'file', key: 'permissions.allow', savedHere: false }),
    ).toEqual({ applies: 'now', configChangeHook: true });
  });

  test('a /model save is in force at once where it was made, without the hook; elsewhere it waits', () => {
    expect(editEffect({ scope: 'user', arrival: 'file', key: 'model', savedHere: true })).toEqual({
      applies: 'now',
      configChangeHook: false,
    });
    expect(editEffect({ scope: 'user', arrival: 'file', key: 'model' })).toEqual({
      applies: 'next-session',
      configChangeHook: true,
    });
  });
});
