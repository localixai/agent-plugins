/**
 * The text a `prompt` or `agent` hook sends (hooks.md, "Prompt and agent hook fields" and "Prompt
 * hook configuration"): "Use `$ARGUMENTS` as a placeholder for the hook input JSON. Escape with a
 * backslash to include literal text: `\$1.00` renders as `$1.00`", and "If `$ARGUMENTS` is not
 * present, input JSON is appended to the prompt".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/prompt_text.rs`, docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('hooks/handlers/remote/prompt-text');

/**
 * The prompt with every `$ARGUMENTS` replaced by `inputJson` and every `\$` by `$`, in one pass so
 * the input JSON is never unescaped or searched for the placeholder again; the input alone is
 * appended when the prompt has no placeholder.
 */
export const composePrompt = (prompt: string, inputJson: string): string =>
  call('composePrompt', prompt, inputJson);
