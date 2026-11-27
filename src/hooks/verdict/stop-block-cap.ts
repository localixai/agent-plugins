import type { Environment } from '../../ports/index.ts';
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';

/**
 * The cap on a Stop hook that keeps blocking (env-vars.md, `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`):
 * "Maximum number of consecutive times a Stop or SubagentStop hook may block the turn from ending
 * before Claude Code overrides it and ends the turn anyway (default: 8). Set to `0` to disable the
 * cap." hooks.md ("Stop decision control") names the same loop protection, "the `stop_hook_active`
 * input and the 8-consecutive-continuation cap", for `additionalContext` that keeps the
 * conversation going too, so both kinds of continuation count.
 *
 * The count is the host's state: one counter per session (and per subagent for `SubagentStop`).
 * hooks.md ("Stop input"): "The count of consecutive continuations resets each time Claude calls a
 * tool", so the engine calls `reset` when a `PreToolUse`, `PostToolUse` or `PostToolUseFailure`
 * event is dispatched; a Stop that does not hold the turn back ends the run of blocks too. A host
 * that builds an engine per run injects a counter that outlives it (`HookEngineOptions.stopCap`)
 * and owns its lifetime: it calls `reset` when the session ends.
 *
 * Judgments (spec/judgments.md): a value that is not a whole number of at least 0 is the default;
 * the block that exceeds the cap is the one overridden (the first 8 count), and the override leaves
 * a warning issue, since the docs describe no message for it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/stop_block_cap.rs`,
 * docs/RUST.md).
 */
const call = coreModule('hooks/verdict/stop-block-cap');

export interface StopBlockCap {
  /** Counts this dispatch and overrides the hold once it exceeds the cap. */
  apply<E extends HookEvent>(
    outcome: HookOutcome<E>,
    where: { readonly sessionId: string; readonly facts: object },
    environment: Environment,
  ): HookOutcome<E>;
  /** Starts the count again for a session (and a subagent): Claude called a tool, or the session ended. */
  reset(sessionId: string, agentId?: string): void;
}

export const createStopBlockCap = (): StopBlockCap => call('createStopBlockCap');
