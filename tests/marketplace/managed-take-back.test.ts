/**
 * Spec §16.31 (the two rows on a marketplace managed settings declare under a name the user's
 * registry holds from another source; plugins/troubleshooting.md): the client keeps the user's
 * marketplace while the declared source is unreachable or `installed_plugins.json` cannot be read,
 * and replaces it in the next interactive session once both are fine.
 */
import { expect, test } from 'bun:test';
import { managedTakeBack } from '../../src/marketplace/managed-take-back.ts';
import { spec } from '../support/spec.ts';

const facts = { name: 'acme', declaredReachable: true, recordsReadable: true, interactive: true };

spec(
  'install.troubleshooting.Marketplace "<name>", which managed settings declare, stands from another source and cannot be taken back while its declared source is unreachable',
  () => {
    test('an unreachable declared source keeps the user marketplace, with the line', () => {
      expect(managedTakeBack({ ...facts, declaredReachable: false })).toEqual({
        replace: false,
        message:
          'Marketplace "acme", which managed settings declare, stands from another source and cannot be taken back while its declared source is unreachable',
      });
    });

    test('once the declared source is reachable, the next interactive session replaces it', () => {
      expect(managedTakeBack(facts)).toEqual({ replace: true });
      expect(managedTakeBack({ ...facts, interactive: false })).toEqual({ replace: false });
    });
  },
);

spec(
  'install.troubleshooting.The list of installed plugins (installed_plugins.json) could not be read, so marketplace "<name>", which managed settings declare, cannot be taken back from another source',
  () => {
    test('an unreadable installed_plugins.json replaces nothing, with the line', () => {
      expect(managedTakeBack({ ...facts, recordsReadable: false })).toEqual({
        replace: false,
        message:
          'The list of installed plugins (installed_plugins.json) could not be read, so marketplace "acme", which managed settings declare, cannot be taken back from another source',
      });
    });
  },
);
