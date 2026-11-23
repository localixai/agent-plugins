/**
 * Spec §14.2.8 `enforceAnthropicSignIn` ("reads the key from a managed settings file, where it
 * applies even beneath server-managed settings; on a machine that also has an MDM policy, from that
 * policy. It ignores the key in server-managed settings, in HKCU and in a `policyHelper`'s output,
 * and still reads it from the MDM policy or file while a helper supplies the rest"); §14.1.1 item 2
 * (a key tied to particular sources); §14.3.5 per-key exceptions; §14.2.5 item 4; §12.5.4 item 3.
 */
import { expect, test } from 'bun:test';
import { withHelperSettings } from '../../../src/settings/managed/index.ts';
import { compose, policyOf } from '../../support/managed.ts';
import { spec } from '../../support/spec.ts';

const ON = { enforceAnthropicSignIn: true } as const;
const MERGE = { managedSourcesBehavior: 'merge' } as const;

spec(
  [
    'policy.managed.Choose a delivery mechanism.Sign-in requirement',
    'policy.server.Per-key exceptions across managed sources.enforceAnthropicSignIn',
  ],
  () => {
    test('the managed settings file supplies it, even beneath server-managed settings', () => {
      expect(policyOf({ file: ON }).enforceAnthropicSignIn).toBe(true);
      expect(policyOf({ remote: { model: 'a' }, file: ON }).enforceAnthropicSignIn).toBe(true);
    });

    test('an MDM policy supplies it', () => {
      for (const mdm of ['plist', 'hklm'] as const) {
        expect(policyOf({ remote: { model: 'a' }, [mdm]: ON }).enforceAnthropicSignIn).toBe(true);
      }
    });

    test('server-managed settings never supply it, even when they are the selected source', () => {
      expect(policyOf({ remote: ON }).enforceAnthropicSignIn).toBeUndefined();
      expect(policyOf({ remote: ON, file: { model: 'a' } }).enforceAnthropicSignIn).toBeUndefined();
      expect(
        policyOf({ remote: { ...ON, ...MERGE }, file: { model: 'a' } }).enforceAnthropicSignIn,
      ).toBeUndefined();
    });

    test('a machine value wins over a server-managed one under merge too', () => {
      const settings = policyOf({
        remote: { ...MERGE, enforceAnthropicSignIn: true },
        file: { enforceAnthropicSignIn: false },
      });
      expect(settings.enforceAnthropicSignIn).toBe(false);
    });

    test('the HKCU registry never supplies it', () => {
      expect(policyOf({ hkcu: { ...ON, model: 'a' } }).enforceAnthropicSignIn).toBeUndefined();
      expect(policyOf({ hkcu: { ...ON, model: 'a' } }).model).toBe('a');
    });

    test("a policyHelper's output does not supply it, and the machine value stays while the helper supplies the rest", () => {
      const helper = { path: '/usr/local/bin/helper' };
      const fromFile = withHelperSettings(compose({ file: { ...ON, policyHelper: helper } }), {
        model: 'b',
      });
      expect(fromFile.layer.settings).toEqual({ model: 'b', enforceAnthropicSignIn: true });
      const fromHelper = withHelperSettings(compose({ file: { policyHelper: helper } }), ON);
      expect(fromHelper.layer.settings.enforceAnthropicSignIn).toBeUndefined();
    });

    test('a value that is not a Boolean is not read', () => {
      expect(policyOf({ file: { enforceAnthropicSignIn: 'yes', model: 'a' } })).toEqual({
        model: 'a',
      });
    });
  },
);
