/**
 * The `schema` option of an `agent()` call (workflows.md, "What the saved script looks like"): Claude
 * Code "checks the schema before starting the subagent: when it can prove the schema contradicts
 * itself, the call fails with an error naming the contradiction", and "if the subagent's output still
 * fails validation after five attempts, the call fails with an error that includes the last validation
 * failure"; `MAX_STRUCTURED_OUTPUT_RETRIES` changes the attempt count (env-vars.md). Pure decisions: the
 * kit validates no output and starts no agent.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/structured_output.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('agents/workflow/structured-output');

/**
 * The contradictions the kit can prove in a `schema` before the subagent starts, each an error that
 * names the key; none means the call goes on. Only the documented one is proven: a `required` key that
 * `additionalProperties: false` rules out, at the top or in a nested property or items schema.
 */
export const checkAgentSchema = (schema: unknown): readonly PluginIssue[] =>
  call('checkAgentSchema', schema);

/**
 * How many attempts a subagent's structured output gets: `MAX_STRUCTURED_OUTPUT_RETRIES` or 5. A value
 * that is not a positive whole number (`2e3`, `64_000` are: J3.17) is no value (judgment, as for the
 * stall window).
 */
export const structuredOutputAttempts = (
  env: Readonly<Record<string, string | undefined>>,
): number => call('structuredOutputAttempts', env);

export type AfterInvalidOutput =
  | { readonly retry: true }
  | { readonly retry: false; readonly issue: PluginIssue };

/**
 * What follows an attempt whose output failed validation: `failures` holds the validation failure of
 * every attempt so far, the last one being the attempt that just ended. The call goes on until the
 * attempts are used up, then fails with an error that includes the last failure (judgment: the wording
 * of the rest).
 */
export const afterInvalidOutput = (
  failures: readonly string[],
  env: Readonly<Record<string, string | undefined>>,
): AfterInvalidOutput => call('afterInvalidOutput', failures, env);
