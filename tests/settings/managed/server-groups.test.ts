/**
 * Spec §14.3.7 Group configurations (server-managed-settings.md, "Give groups different settings"):
 * the highest-ranked configuration among a member's groups, else the catch-all, else none in local
 * sessions; configurations never merge; the organization-wide configuration until group-based mode
 * (and not with HIPAA); a group's `availableModels` holds only in local sessions.
 */
import { expect, test } from 'bun:test';
import { deliveredConfiguration } from '../../../src/settings/managed/index.ts';
import { spec } from '../../support/spec.ts';

const ORG = { model: 'org', availableModels: ['org-model'] };
const delivery = {
  groupBased: true,
  organization: ORG,
  groups: [
    { group: 'security', settings: { model: 'sec', availableModels: ['sec-model'] } },
    { group: 'eng', settings: { model: 'eng' } },
  ],
  catchAll: { model: 'all', availableModels: ['all-model'] },
};
const LOCAL = { local: true };

spec('policy.server.Give groups different settings', () => {
  test('a member receives the highest-ranked configuration among their groups, whole', () => {
    expect(deliveredConfiguration(delivery, { groups: ['eng', 'security'] }, LOCAL)).toEqual({
      model: 'sec',
      availableModels: ['sec-model'],
    });
    expect(deliveredConfiguration(delivery, { groups: ['eng'] }, LOCAL)).toEqual({ model: 'eng' });
  });

  test('anyone else receives the catch-all, a member of a deleted group included', () => {
    expect(deliveredConfiguration(delivery, { groups: ['gone'] }, LOCAL)).toEqual(
      delivery.catchAll,
    );
  });

  test('with no catch-all, no server-managed settings in a local session', () => {
    const noCatchAll = { ...delivery, catchAll: undefined };
    expect(deliveredConfiguration(noCatchAll, { groups: [] }, LOCAL)).toBeUndefined();
  });

  test('until group-based mode every member keeps the organization-wide configuration', () => {
    const before = { ...delivery, groupBased: false };
    expect(deliveredConfiguration(before, { groups: ['security'] }, LOCAL)).toEqual(ORG);
  });

  test("a group's availableModels holds only locally: cloud sessions take the catch-all's, else the organization-wide one's", () => {
    const cloud = { local: false };
    expect(deliveredConfiguration(delivery, { groups: ['security'] }, cloud)).toEqual({
      model: 'sec',
      availableModels: ['all-model'],
    });
    const noCatchAll = { ...delivery, catchAll: undefined };
    expect(deliveredConfiguration(noCatchAll, { groups: ['security'] }, cloud)).toEqual({
      model: 'sec',
      availableModels: ['org-model'],
    });
    const neither = { ...noCatchAll, organization: { model: 'org' } };
    expect(deliveredConfiguration(neither, { groups: ['security'] }, cloud)).toEqual({
      model: 'sec',
    });
  });
});
