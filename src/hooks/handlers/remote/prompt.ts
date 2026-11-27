/**
 * A `prompt` hook (hooks.md, "Prompt-based hooks"): the hook input and the prompt go to a model
 * and "the LLM responds with structured JSON containing a decision". What `ok: false` does on an
 * event is `ok-false.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/prompt.rs`,
 * docs/RUST.md).
 */
import type { HandlerEntry, HandlerResult, RunContext } from '../context.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('hooks/handlers/remote/prompt');

export const runPrompt = (
  entry: HandlerEntry<'prompt'>,
  context: RunContext,
): Promise<HandlerResult> => call.async('runPrompt', entry, context);
