/**
 * The answer of a `prompt` or `agent` hook (hooks.md, "Response schema"):
 * `{ "ok": true | false, "reason": "...", "impossible": true | false }`. `reason` is "required when
 * `ok` is `false`"; `impossible` is optional and "agent hooks ... ignore it". A reply wrapped in one
 * Markdown code fence ("```json ... ```") is read as the JSON inside.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/prompt_response.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';
import type { ModelRun } from './types.ts';

const call = coreModule('hooks/handlers/remote/prompt-response');

/** What reading a reply gives: the verdict, or why the reply is not one. */
export type ParsedVerdict = { readonly verdict: ModelRun } | { readonly failure: string };

/**
 * Judgment (spec/judgments.md): a reply that is not exactly one JSON object of this schema (prose
 * around it, a missing `ok`, `ok: false` with no reason) is no verdict, so the hook ends as a
 * non-blocking error instead of allowing or blocking on a guess.
 */
export const parseVerdict = (
  text: string,
  options: { readonly impossible: boolean },
): ParsedVerdict => call('parseVerdict', text, options);

/**
 * What the model is told about its answer: the doc gives the schema and the port a model call is
 * only a prompt, so the instruction travels with it (judgment, spec/judgments.md).
 */
export const responseInstruction = (options: { readonly impossible: boolean }): string =>
  call('responseInstruction', options);
