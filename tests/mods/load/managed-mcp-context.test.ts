/**
 * plugins/mods/admin.md § Know what happens by default ("A built-in guard runs first" on a machine
 * with managed settings or a `managed-mcp.json`, or a Team or Enterprise `/login`); plugins/mods/
 * reference.md § Settings and environment variables (`prependPlugins` and `appendPlugins` in user
 * settings count only with no managed settings, no `managed-mcp.json` and no such login);
 * permissions.md § Precedence among rules, hooks, and mods. A machine whose only managed file is
 * `managed-mcp.json` (`ModsPolicyInput.managedMcp`).
 */
import { expect, test } from 'bun:test';
import { listSetting } from '../../../src/mods/load/policy-input.ts';
import { installed, modPlugin, policy, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

const registerCalls = registerOf(() => undefined);

spec('policy.mods.Know what happens by default.A built-in guard runs first', () => {
  test('a machine with only managed-mcp.json loads the guard first', async () => {
    const { host } = session({ first: registerCalls }, { policy: policy({ managedMcp: true }) });
    const report = await host.load([modPlugin(installed('first'), registerCalls)]);
    expect(report.loaded.map((mod) => mod.name)).toEqual(['cc-plugin-sec-default', 'first']);
  });
});

spec(['mods.setting.prependPlugins', 'mods.setting.appendPlugins'], () => {
  const user = { prependPlugins: ['mine@acme'], appendPlugins: ['late@acme'] };

  test("with managed-mcp.json the user's lists do not count", () => {
    const mcp = policy({ managedMcp: true, user });
    expect(listSetting(mcp, 'prependPlugins')).toBeUndefined();
    expect(listSetting(mcp, 'appendPlugins')).toBeUndefined();
  });

  test('without it they do', () => {
    expect(listSetting(policy({ user }), 'prependPlugins')?.ids).toEqual(['mine@acme']);
  });
});
