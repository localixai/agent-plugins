import { coreModule } from '../wasm/bridge.ts';

/**
 * What a user's interrupt does to the hooks of an event (hooks.md, "PostToolUse after an
 * interrupt" and "`Esc` during a prompt check").
 *
 * - `PostToolUse`: `Ctrl+Enter` that ends the turn while a tool runs gives the hooks "10 seconds to
 *   finish" and kills a command hook still running then, "whatever its `timeout`"; `Enter` only
 *   queues a message, so "the hooks keep their usual `timeout`"; `Esc` cancels the hooks at once.
 *   `Ctrl+Enter` "acts as `Esc` if you queued a `!` command ahead of your messages or background
 *   tasks are disabled, for example with `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`".
 * - `UserPromptSubmit`: an interrupt while the hooks check a prompt stops the check, the prompt
 *   does not reach Claude, and the text goes back to the input, unless a hook blocked the prompt
 *   at the same moment: then it "stays blocked" and the text does not come back.
 *
 * Judgments (spec/judgments.md): the first rule is read for `PostToolUse` only, since no other
 * event's section says so; the second is read from the request's abort signal (an interrupted
 * session aborts it) together with a hook that was cancelled by it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/interrupt.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/interrupt');

export const INTERRUPT_KEYS: readonly ['esc', 'enter', 'ctrl-enter'] = call('INTERRUPT_KEYS');
export type InterruptKey = (typeof INTERRUPT_KEYS)[number];

/** The key the user pressed while a tool ran, and what the host knows of what they queued. */
export interface UserInterrupt {
  readonly key: InterruptKey;
  /** A `!` shell command was queued ahead of the messages. */
  readonly queuedShellCommand?: boolean;
}
