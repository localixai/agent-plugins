/**
 * hooks.md § Hooks in skills and agents ("Frontmatter hooks in a project skill follow the same
 * workspace trust rule as hooks in settings files. Claude Code registers them when you or Claude
 * invoke the skill, including in a `-p` run in a folder you haven't trusted") and § Limits on skill
 * hooks (under a plugin-only lock on hooks a skill keeps its frontmatter hooks only as "a plugin
 * skill, a bundled skill, or a skill in the managed settings directory"). Proven through the real
 * registration path: a folder skill expanded by a session, not a hand-built source. Under
 * `allowManagedHooksOnly` a managed-directory skill's hooks run and a personal one's do not: a
 * judgment (spec/judgments.md, `src/hooks/select/lock-gates.ts`), tested outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import type { PluginSession } from '../../src/host/index.ts';
import { createPolicyView } from '../../src/settings/index.ts';
import { signal } from '../support/host.ts';
import { HOME, MANAGED, PROJECT, folderHostOf } from '../support/scan.ts';
import { layerOf } from '../support/settings-layers.ts';
import { spec } from '../support/spec.ts';

const SKILL = [
  '---',
  'description: Guarded operations',
  'hooks:',
  '  PreToolUse:',
  '    - matcher: "Bash"',
  '      hooks:',
  '        - type: command',
  '          command: ./scripts/check.sh',
  '---',
  'Run guarded.',
].join('\n');

const FOLDERS = {
  project: `${PROJECT}/.claude/skills/guarded/SKILL.md`,
  user: `${HOME}/.claude/skills/guarded/SKILL.md`,
  managed: `${MANAGED}/.claude/skills/guarded/SKILL.md`,
} as const;

type Folder = keyof typeof FOLDERS;

const bash = { tool_name: 'Bash', tool_input: { command: 'ls' }, tool_use_id: 't1' };

const managedSettings = (settings: Record<string, unknown>) =>
  createPolicyView([layerOf('managed', settings)]);

interface Case {
  readonly folder: Folder;
  readonly trusted?: boolean;
  readonly settings?: Record<string, unknown>;
}

/** How many times the skill's hook ran for one Bash call after the skill was invoked. */
async function runsAfterInvoking(scenario: Case): Promise<number> {
  const { session, runner } = folderHostOf(
    { [FOLDERS[scenario.folder]]: SKILL },
    {
      runs: [{}],
      session: { trusted: scenario.trusted ?? true },
      ...(scenario.settings === undefined ? {} : { settings: managedSettings(scenario.settings) }),
    },
  );
  await session.reloadSkills({ signal: signal() });
  await invoke(session);
  await session.dispatch('PreToolUse', bash, { signal: signal() });
  return runner.calls.length;
}

async function invoke(session: PluginSession): Promise<void> {
  const expansion = await session.expandSkill('guarded', { signal: signal() });
  expect(expansion.ok).toBe(true);
}

spec('hooks.behavior.Hooks in skills and agents', () => {
  test("a project skill's hooks are held back in an untrusted interactive session", async () => {
    expect(await runsAfterInvoking({ folder: 'project', trusted: false })).toBe(0);
  });

  test('once the folder is trusted, the same skill runs its hook', async () => {
    expect(await runsAfterInvoking({ folder: 'project', trusted: true })).toBe(1);
  });

  test("a personal skill's hooks do not wait for the project's trust", async () => {
    expect(await runsAfterInvoking({ folder: 'user', trusted: false })).toBe(1);
  });
});

spec('hooks.behavior.Limits on skill hooks', () => {
  const lock = { strictPluginOnlyCustomization: ['hooks'] };

  test('under a plugin-only lock on hooks, a project or personal skill registers none of its hooks', async () => {
    expect(await runsAfterInvoking({ folder: 'project', settings: lock })).toBe(0);
    expect(await runsAfterInvoking({ folder: 'user', settings: lock })).toBe(0);
  });

  test('a skill in the managed settings directory keeps its hooks under that lock', async () => {
    expect(await runsAfterInvoking({ folder: 'managed', settings: lock })).toBe(1);
  });
});

describe('allowManagedHooksOnly and frontmatter hooks (judgment)', () => {
  const only = { allowManagedHooksOnly: true };

  test("a managed-directory skill's hooks are managed hooks and run", async () => {
    expect(await runsAfterInvoking({ folder: 'managed', settings: only })).toBe(1);
  });

  test("a personal or project skill's hooks are blocked", async () => {
    expect(await runsAfterInvoking({ folder: 'user', settings: only })).toBe(0);
    expect(await runsAfterInvoking({ folder: 'project', settings: only })).toBe(0);
  });
});
