/**
 * What the permission check (`decide`) reads of the `PreToolUse` hooks: their merged
 * `permissionDecision`, whether a hook exited with code 2 ("stops the tool call before permission
 * rules are evaluated"), and whether the answer carries `updatedInput`, which `AskUserQuestion` and
 * `ExitPlanMode` need beside an `allow` (hooks.md, "Tools that require user interaction").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/pre_tool_use_outcome.rs`, docs/RUST.md).
 */
import type { PreToolUseHookOutcome } from '../../rules/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';

const call = coreModule('hooks/output/pre-tool-use-outcome');

export const preToolUseHookOutcome = (
  outcome: Pick<HookOutcome<'PreToolUse'>, 'block' | 'effects'>,
): PreToolUseHookOutcome => call('preToolUseHookOutcome', outcome);
