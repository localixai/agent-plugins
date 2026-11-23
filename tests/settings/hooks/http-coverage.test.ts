/**
 * Spec §12.7.1 `allowedHttpHookUrls` and Judgment J12.18: "defined in managed settings it covers the
 * hooks of managed settings, defined in any file it covers every other HTTP hook. A covered hook runs
 * only if its URL matches a pattern of the list merged across files"; a user entry can admit a
 * managed hook's URL once a managed list exists.
 */
import { describe, expect, test } from 'bun:test';
import { plain } from '../../support/settings-layers.ts';
import {
  fromManaged,
  fromSettings,
  hookPolicyOf,
  managed,
  runs,
  user,
} from '../../support/hook-policy.ts';
import { spec } from '../../support/spec.ts';

const allowed = (policy: ReturnType<typeof hookPolicyOf>, url: string): boolean =>
  policy.httpHook(url, []).run;

spec('settings.key.allowedHttpHookUrls', () => {
  describe('which hooks a list covers (spec §12.7.1, Judgment J12.18)', () => {
    const managedHook = (policy: ReturnType<typeof hookPolicyOf>, url: string): boolean =>
      policy.httpHook(url, [], { managed: true }).run;

    test('a managed list covers the hooks of managed settings: runs(managed) stays true, the URL is still checked', () => {
      const policy = hookPolicyOf([
        managed({ allowManagedHooksOnly: true, allowedHttpHookUrls: ['https://a.test/*'] }),
      ]);
      expect(runs(policy, fromManaged)).toBe(true);
      expect(plain(policy.httpHook('https://managed-hook.test/', [], { managed: true }))).toEqual({
        run: false,
        message: expect.any(String),
      });
      expect(managedHook(policy, 'https://a.test/x')).toBe(true);
    });

    test('a list in any file, managed included, covers every other hook', () => {
      const policy = hookPolicyOf([managed({ allowedHttpHookUrls: ['https://a.test/*'] })]);
      expect(runs(policy, fromSettings('user'))).toBe(true);
      expect(allowed(policy, 'https://user-hook.test/')).toBe(false);
      expect(allowed(policy, 'https://a.test/x')).toBe(true);
    });

    test('a list only outside managed settings leaves the hooks of managed settings uncovered', () => {
      const policy = hookPolicyOf([
        managed({}),
        user({ allowedHttpHookUrls: ['https://mine.test/*'] }),
      ]);
      expect(managedHook(policy, 'https://org.test/hook')).toBe(true);
      expect(allowed(policy, 'https://org.test/hook')).toBe(false);
    });

    test('once a managed list exists, a user entry admits a managed hook: the merged list applies', () => {
      const policy = hookPolicyOf([
        managed({ allowedHttpHookUrls: ['https://org.test/*'] }),
        user({ allowedHttpHookUrls: ['https://mine.test/*'] }),
      ]);
      expect(managedHook(policy, 'https://mine.test/x')).toBe(true);
      expect(managedHook(policy, 'https://org.test/x')).toBe(true);
      expect(managedHook(policy, 'https://else.test/x')).toBe(false);
    });

    test('an empty managed list covers the managed hooks and blocks them unless another file lists the URL', () => {
      const policy = hookPolicyOf([
        managed({ allowedHttpHookUrls: [] }),
        user({ allowedHttpHookUrls: ['https://mine.test/*'] }),
      ]);
      expect(managedHook(policy, 'https://org.test/x')).toBe(false);
      expect(managedHook(policy, 'https://mine.test/x')).toBe(true);
    });
  });
});
