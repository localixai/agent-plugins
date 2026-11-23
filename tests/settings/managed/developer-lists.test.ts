/**
 * Spec §14.6, "Allow rules and allowlists" (managed-settings.md § What a developer can change): "A
 * developer can add entries to lists such as `permissions.allow` and `allowedMcpServers`, which
 * merge with the managed ones (§12.4); only the list's `allowManaged*Only` key, where it has one,
 * confines it to managed entries, and under `allowManagedPermissionRulesOnly` the user's deny and ask
 * rules stop applying too, so the managed source carries those it relies on."
 */
import { expect, test } from 'bun:test';
import { readMcpLists } from '../../../src/settings/mcp/lists.ts';
import { permissionsOf, ruleLines } from '../../support/permissions-policy.ts';
import { viewOf } from '../../support/plugin-settings.ts';
import { spec } from '../../support/spec.ts';

const names = (view: ReturnType<typeof viewOf>) =>
  readMcpLists(view).allowed?.serverName.map((entry) => entry.value);

spec('policy.managed.What a developer can change.Allow rules and allowlists', () => {
  test("a developer's allow rules merge with the managed ones", () => {
    const policy = permissionsOf([
      ['managed', { permissions: { allow: ['Bash(npm test)'] } }],
      ['user', { permissions: { allow: ['Bash(ls)'] } }],
    ]);
    expect(ruleLines(policy)).toEqual(['allow:Bash(npm test)@managed', 'allow:Bash(ls)@user']);
  });

  test("a developer's allowedMcpServers entries merge with the managed list", () => {
    const view = viewOf(
      ['managed', { allowedMcpServers: [{ serverName: 'org' }] }],
      ['user', { allowedMcpServers: [{ serverName: 'mine' }] }],
    );
    expect(names(view)).toEqual(['org', 'mine']);
  });

  test("the list's allowManaged*Only key confines it to the managed entries", () => {
    const view = viewOf(
      ['managed', { allowManagedMcpServersOnly: true, allowedMcpServers: [{ serverName: 'org' }] }],
      ['user', { allowedMcpServers: [{ serverName: 'mine' }] }],
    );
    expect(names(view)).toEqual(['org']);
  });

  test("under allowManagedPermissionRulesOnly the developer's deny and ask rules stop applying too", () => {
    const policy = permissionsOf([
      ['managed', { allowManagedPermissionRulesOnly: true, permissions: { deny: ['Bash(rm *)'] } }],
      [
        'user',
        { permissions: { allow: ['Bash(ls)'], deny: ['Read(.env)'], ask: ['Bash(git *)'] } },
      ],
    ]);
    expect(ruleLines(policy)).toEqual(['deny:Bash(rm *)@managed']);
  });
});
