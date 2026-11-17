import type { BackgroundResult, ForegroundRun } from '../handlers/context.ts';
import { readAsyncResponse } from './stdout-kind.ts';
import type { BackgroundDelivery, BackgroundOutcome } from './types.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { warning, type PluginIssue } from '../../format/issues.ts';

/**
 * What a background hook hands over once its process ends (hooks.md, "How async hooks execute"):
 * "Claude Code delivers the `additionalContext` and `systemMessage` fields from the hook's JSON
 * response to Claude on the next conversation turn". The response is validated and "any field whose
 * value has the wrong type, such as a `systemMessage` that isn't a string" is dropped. Nothing else
 * of the response matters: "response fields like `decision`, `permissionDecision`, and `continue`
 * have no effect, because the action they would have controlled has already completed". An
 * `asyncRewake` hook that exited with code 2 "wakes Claude" with its stderr or stdout.
 */

const textOf = (value: unknown): string | undefined =>
  typeof value === 'string' && value !== '' ? value : undefined;

/**
 * "drops each field of the wrong type instead of delivering it, with a debug warning naming the
 * field" (Agent Plugins Extended §7.3.3 item 3): a delivered field that is present and not a string.
 */
function droppedField(handlerId: string, field: string, value: unknown): readonly PluginIssue[] {
  return value === undefined || typeof value === 'string'
    ? []
    : [
        warning(
          `hooks.${handlerId}.${field}`,
          `The async hook response field ${field} is not a string and is dropped`,
        ),
      ];
}

const OUTCOMES: { readonly [K in ForegroundRun['kind']]: BackgroundOutcome } = {
  exited: 'completed',
  'timed-out': 'timed-out',
  'not-started': 'failed',
  cancelled: 'cancelled',
};

/** Process metadata stays apart from the response fields a hook printed. */
const completionOf = (run: ForegroundRun) => ({
  outcome: OUTCOMES[run.kind],
  ...('exitCode' in run ? { exitCode: run.exitCode } : {}),
});

export function deliveryOf(handlerId: string, result: BackgroundResult): BackgroundDelivery {
  const response = 'stdout' in result.run ? readAsyncResponse(result.run.stdout) : undefined;
  const specific =
    response !== undefined && 'hookSpecificOutput' in response
      ? response.hookSpecificOutput
      : undefined;
  const givenContext = isRecord(specific) ? specific.additionalContext : undefined;
  const givenMessage = isRecord(response) ? response.systemMessage : undefined;
  const additionalContext = textOf(givenContext);
  const systemMessage = textOf(givenMessage);
  const issues = [
    ...droppedField(handlerId, 'hookSpecificOutput.additionalContext', givenContext),
    ...droppedField(handlerId, 'systemMessage', givenMessage),
  ];
  return {
    handlerId,
    ...completionOf(result.run),
    ...(additionalContext === undefined ? {} : { additionalContext }),
    ...(systemMessage === undefined ? {} : { systemMessage }),
    ...(result.wake === undefined ? {} : { wake: result.wake }),
    ...(issues.length === 0 ? {} : { issues }),
  };
}
