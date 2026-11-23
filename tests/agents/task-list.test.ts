/**
 * Agent Plugins Extended §5.8.2 ("Checking": `/tasks` names each subagent's model and effort) and
 * §5.12.3 (the row that reopens ended subagents); sub-agents.md, "Check a subagent's model";
 * env-vars.md, `CLAUDE_CODE_SUBAGENT_HISTORY`.
 */
import { expect, test } from 'bun:test';
import { resolveSubagentEffort, resolveSubagentModel } from '../../src/agents/launch-model.ts';
import { offersSubagentHistory, subagentTaskRow } from '../../src/agents/task-list.ts';
import { spec } from '../support/spec.ts';

spec("agents.behavior.Check a subagent's model", () => {
  test('a row names the model the subagent runs on and the effort level set for it', () => {
    const { model } = resolveSubagentModel({ definition: 'haiku', mainModel: 'claude-opus-5-5' });
    const effort = resolveSubagentEffort({ field: 'low', sameModelAsMain: false });
    expect(subagentTaskRow({ model, effort })).toEqual({ model: 'haiku', effort: 'low' });
  });

  test('with no effort level set, the row names the model alone', () => {
    const effort = resolveSubagentEffort({ session: 'high', sameModelAsMain: false });
    expect(subagentTaskRow({ model: 'haiku', effort })).toEqual({ model: 'haiku' });
  });
});

spec('environment.host.CLAUDE_CODE_SUBAGENT_HISTORY', () => {
  test('0 removes the row that reopens ended subagents; unset keeps it', () => {
    expect(offersSubagentHistory(undefined)).toBe(true);
    expect(offersSubagentHistory('1')).toBe(true);
    expect(offersSubagentHistory('0')).toBe(false);
  });
});
