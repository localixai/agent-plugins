/**
 * Judgments (spec/judgments.md) of the permission-mode resolution: where the docs are silent the
 * kit decides, and says so. Not under spec(): never counted as compatibility.
 */
import { describe, expect, test } from 'bun:test';
import { baselineDecision } from '../../src/agents/permission-baseline.ts';
import { resolveSubagentPermissionMode } from '../../src/agents/subagent-permission.ts';

describe('a permissionMode that is no mode counts as unset', () => {
  test('the subagent inherits the main conversation mode', () => {
    // A host passing raw text: readAgent already drops such a value with a warning.
    const set: string = 'yolo';
    expect(resolveSubagentPermissionMode({ main: 'default', set, autoAvailable: true })).toEqual({
      mode: 'default',
      denyInsteadOfAsk: false,
      basis: 'inherited',
    });
  });
});

describe('a read outside the working directories', () => {
  test('asks in default, acceptEdits, plan and auto (§13.1 item 2, §13.9.4 item 2); only the first such read in auto is session state, not modelled', () => {
    for (const mode of ['default', 'acceptEdits', 'plan', 'auto'] as const) {
      expect(baselineDecision(mode, { category: 'read', inScope: false })).toBe('ask');
      expect(baselineDecision(mode, { category: 'read', inScope: true })).toBe('allow');
    }
  });
});

describe('an agent team teammate is not resolved here', () => {
  test('the plan rule applies to a subagent; a teammate follows the agent teams rules of its host', () => {
    // The function has no teammate input: it answers for subagents only.
    expect(
      resolveSubagentPermissionMode({ main: 'plan', set: 'default', autoAvailable: true }).mode,
    ).toBe('plan');
  });
});
