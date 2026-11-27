/**
 * The dispatch wires the stages of `contracts.ts` and nothing else: with a fake output stage it
 * selects, builds the input, runs every handler with its own input text, reads the runs and
 * hands the merge what each hook said (hooks.md § "How a hook resolves": event, matcher, handler,
 * result). Judgments of the wiring are tested here, outside `spec(...)`.
 */
import { expect, test } from 'bun:test';
import {
  createHookEngine,
  createHookRegistry,
  type OutputStage,
  type TaggedOutput,
} from '../../src/hooks/index.ts';
import { entry, FACTS, session } from '../support/hooks-select.ts';
import { fakeOutput } from '../support/hooks-verdict.ts';
import { fakeEnvironment, locations, scriptedRunner } from '../support/ports.ts';

test("the merge receives each hook's output, and a Monitor call's effects flagged as ignored", async () => {
  const hook = entry('PreToolUse', {
    matcher: 'Bash',
    handler: { type: 'command', command: './a.sh' },
  });
  const seen: TaggedOutput[] = [];
  const runner = scriptedRunner({ exitCode: 0, stdout: '{"systemMessage":"x"}', stderr: '' });
  const output: OutputStage = {
    parse: fakeOutput.parse,
    limit: (event, outputs, signal) => fakeOutput.limit(event, outputs, signal),
    merge: (event, outputs, facts) => {
      seen.push(...outputs);
      return fakeOutput.merge(event, outputs, facts);
    },
  };
  const engine = createHookEngine(
    { commands: runner, locations: locations(), environment: fakeEnvironment() },
    { output },
  );
  const outcome = await engine.dispatch(createHookRegistry([hook]), {
    event: 'PreToolUse',
    facts: {
      ...FACTS.PreToolUse,
      tool_name: 'Monitor',
      tool_input: { description: 'tail', timeout_ms: 1000, command: 'tail -f app.log' },
    },
    session: session(),
    signal: new AbortController().signal,
  });
  expect(JSON.parse(runner.calls[0]?.stdin ?? '{}').tool_name).toBe('Bash');
  expect(seen).toHaveLength(1);
  expect(seen[0]?.effectsIgnored).toBe(true);
  expect(outcome.systemMessages).toEqual(['x']);
});
