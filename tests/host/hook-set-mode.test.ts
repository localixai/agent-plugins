/**
 * hooks.md § Permission update entries ("`setMode` with `bypassPermissions` only takes effect if you
 * launched the session with bypass mode already available: `--dangerously-skip-permissions`,
 * `--permission-mode bypassPermissions`, `--allow-dangerously-skip-permissions`, or
 * `permissions.defaultMode: "bypassPermissions"` ... Otherwise the update is a no-op. The update is
 * also a no-op when `permissions.disableBypassPermissionsMode` disables the mode, or when the session
 * starts in restricted mode"; "`bypassPermissions` is never persisted as `defaultMode` regardless of
 * `destination`"). Asserted over the session `startPermissions` builds and through a plugin session's
 * `PermissionRequest` dispatch. The session's fallback when the host gives no `permissionContext` is
 * a judgment (spec/judgments.md), tested outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import type { PermissionMode, PermissionUpdate } from '../../src/protocol/official.ts';
import { permissionUpdatesInEffect } from '../../src/rules/index.ts';
import { hostOf, loaded, SESSION, signal } from '../support/host.ts';
import { sessionOf } from '../support/permission-session.ts';
import { permissionsOf, terminal } from '../support/permissions-policy.ts';
import { spec } from '../support/spec.ts';

const toBypass: PermissionUpdate = {
  type: 'setMode',
  mode: 'bypassPermissions',
  destination: 'userSettings',
};
const toEdits: PermissionUpdate = {
  type: 'setMode',
  mode: 'acceptEdits',
  destination: 'localSettings',
};
const rule: PermissionUpdate = {
  type: 'addRules',
  rules: [{ toolName: 'Bash', ruleContent: 'npm test' }],
  behavior: 'allow',
  destination: 'projectSettings',
};

const none = permissionsOf([]);
const inEffect = (start = terminal(), policy = none) =>
  permissionUpdatesInEffect([rule, toBypass, toEdits], sessionOf(policy, start).context);

spec('hooks.permissionUpdate.setMode', () => {
  test('without bypass mode available the bypass entry is a no-op; the others stay', () => {
    expect(inEffect()).toEqual([rule, toEdits]);
  });

  test('a session launched with bypass available takes it, in memory only', () => {
    const asked = inEffect(terminal({ allowDangerouslySkipPermissions: true }));
    expect(asked).toEqual([rule, { ...toBypass, destination: 'session' }, toEdits]);
    const started = inEffect(terminal({ permissionMode: 'bypassPermissions' }));
    expect(started).toContainEqual({ ...toBypass, destination: 'session' });
  });

  test('disableBypassPermissionsMode and restricted mode make it a no-op', () => {
    const disabled = permissionsOf([
      ['managed', { permissions: { disableBypassPermissionsMode: 'disable' } }],
    ]);
    expect(inEffect(terminal({ allowDangerouslySkipPermissions: true }), disabled)).toEqual([
      rule,
      toEdits,
    ]);
    expect(inEffect(terminal({ allowDangerouslySkipPermissions: true, restricted: true }))).toEqual(
      [rule, toEdits],
    );
  });
});

async function dispatched(session: { readonly permissionMode?: PermissionMode } = {}) {
  const plugin = await loaded({
    '.claude-plugin/plugin.json': JSON.stringify({ name: 'moder' }),
    'hooks/hooks.json': JSON.stringify({
      hooks: { PermissionRequest: [{ hooks: [{ type: 'command', command: './allow.sh' }] }] },
    }),
  });
  const output = {
    hookSpecificOutput: {
      hookEventName: 'PermissionRequest',
      decision: { behavior: 'allow', updatedPermissions: [toBypass, toEdits] },
    },
  };
  const { host } = await hostOf({
    plugins: [{ id: 'moder@acme', plugin }],
    runs: [{ exitCode: 0, stdout: JSON.stringify(output) }],
  });
  const outcome = await host
    .openSession({ ...SESSION, ...session })
    .dispatch(
      'PermissionRequest',
      { tool_name: 'Bash', tool_input: { command: 'ls' } },
      { signal: signal() },
    );
  return outcome.effects.permissionRequest;
}

describe("a session's PermissionRequest dispatch hands on only what takes effect (judgment for the fallback)", () => {
  test('a session given no permissionContext, not in bypass mode, drops the bypass entry', async () => {
    expect(await dispatched()).toEqual({ behavior: 'allow', updatedPermissions: [toEdits] });
  });

  test('a session already in bypassPermissions keeps it in memory', async () => {
    expect(await dispatched({ permissionMode: 'bypassPermissions' })).toEqual({
      behavior: 'allow',
      updatedPermissions: [{ ...toBypass, destination: 'session' }, toEdits],
    });
  });
});
