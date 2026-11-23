/**
 * Agent Plugins Extended §7.5.13 `PostToolUse`, output `updatedToolOutput` (2.1.330): "In a Cowork
 * cloud session the client ignores it for `AskUserQuestion` and `ExitPlanMode`, and Claude reads the
 * original answer or plan" (`DispatchRequest.coworkCloud`). Every other tool, and both tools outside
 * such a session, keep the hook's replacement. Through the real engine and output stage.
 */
import { describe, expect, test } from 'bun:test';
import { createHookEngine, createHookRegistry } from '../../../src/hooks/index.ts';
import { command, entry, FACTS, session } from '../../support/hooks-select.ts';
import { fakeEnvironment, locations, scriptedRunner } from '../../support/ports.ts';
import { spec } from '../../support/spec.ts';

const ANSWER = {
  questions: [{ question: 'Ship?', header: 'Ship', options: [], multiSelect: false }],
  answers: { 'Ship?': 'No' },
};
const PLAN = { plan: 'Replaced plan', isAgent: false };
const BASH = { stdout: 'replaced', stderr: '', interrupted: false, isImage: false };

async function rewritten(toolName: string, replacement: unknown, coworkCloud?: boolean) {
  const stdout = JSON.stringify({
    hookSpecificOutput: { hookEventName: 'PostToolUse', updatedToolOutput: replacement },
  });
  const engine = createHookEngine({
    commands: scriptedRunner({ exitCode: 0, stdout }),
    locations: locations(),
    environment: fakeEnvironment(),
  });
  const outcome = await engine.dispatch(
    createHookRegistry([entry('PostToolUse', { handler: command('./rewrite.sh') })]),
    {
      event: 'PostToolUse',
      facts: { ...FACTS.PostToolUse, tool_name: toolName },
      session: session(),
      signal: new AbortController().signal,
      ...(coworkCloud === undefined ? {} : { coworkCloud }),
    },
  );
  return outcome.effects.updatedToolOutput;
}

spec('hooks.output.PostToolUse.updatedToolOutput', () => {
  describe('in a Cowork cloud session', () => {
    test('AskUserQuestion keeps the original answer', async () => {
      expect(await rewritten('AskUserQuestion', ANSWER, true)).toBeUndefined();
    });

    test('ExitPlanMode keeps the original plan', async () => {
      expect(await rewritten('ExitPlanMode', PLAN, true)).toBeUndefined();
    });

    test('another tool still takes the replacement', async () => {
      expect(await rewritten('Bash', BASH, true)).toEqual(BASH);
    });
  });

  test('outside one, AskUserQuestion and ExitPlanMode take the replacement', async () => {
    expect(await rewritten('AskUserQuestion', ANSWER)).toMatchObject(ANSWER);
    expect(await rewritten('ExitPlanMode', PLAN, false)).toMatchObject(PLAN);
  });
});
