/**
 * The reasons a blocked model switch is reported with when no hook wrote one, and the report line
 * (errors.md, "Model switch was blocked by a PreModelSwitch hook"; Agent Plugins Extended §7.5.33).
 * The timeout reason is `timedOutBlocksSwitch` (notices.ts), carried by the hook's block.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/model_switch.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/verdict/model-switch');

export const MODEL_SWITCH_REASONS: {
  /** The hook run ended without a verdict because a hook failed: never taken as approval. */
  readonly failed: 'a PreModelSwitch hook failed before answering';
  /** The run ended without a verdict because the control stream closed. */
  readonly cancelled: 'PreModelSwitch hooks were cancelled (the control stream closed) before answering';
} = call('MODEL_SWITCH_REASONS');

/**
 * `confirmation required, and this session cannot ask`: an `ask` without a reason where no
 * confirmation can be shown; `/model` in a `-p` run adds `(run /model interactively to confirm)`.
 */
export const cannotAskReason = (modelCommandInPrint = false): string =>
  call('cannotAskReason', modelCommandInPrint);

/**
 * The client cannot tell which `PreModelSwitch` hooks managed plugins deliver (a managed plugin
 * failed to load): the reason begins with what failed and ends with the fixed clause.
 */
export const managedHooksUncheckedReason = (whatFailed: string): string =>
  call('managedHooksUncheckedReason', whatFailed);

/**
 * The report: `Model switch to <model> was blocked by a PreModelSwitch hook: <reason>` for a switch
 * the user typed or picked; `Model switch blocked by a PreModelSwitch hook: <reason>`, not naming
 * the model, for one an Agent SDK host or Remote Control requested (`model` left out).
 */
export const modelSwitchBlocked = (reason: string, model?: string): string =>
  call('modelSwitchBlocked', reason, model);
