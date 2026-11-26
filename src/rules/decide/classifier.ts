import type { PermissionBehavior, PermissionMode } from '../../protocol/official.ts';
import type { Decision, PreparedCall } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PromptFacts } from './unanswered.ts';

const call = coreModule('rules/decide/classifier');

/**
 * The host's reviewer for auto mode (permission-modes.md, "Eliminate permission prompts with auto
 * mode"): "a second model, the classifier, reviews actions instead of you". The kit holds no model
 * and no classifier. `decide` returns `classify` for an action that goes to the reviewer; the host
 * (or `settleClassification`) hands it here.
 *
 * It names `PreparedCall`, a layer-3 type, so it lives in the rules module rather than in
 * `src/ports/` (docs/ARCHITECTURE.md, "Ports").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/classifier.rs`, docs/RUST.md).
 */
export interface PermissionClassifier {
  review(
    call: PreparedCall,
    context: { readonly mode: PermissionMode },
    signal?: AbortSignal,
  ): Promise<PermissionBehavior>;
}

/** What a host hands `settleClassification`: the call, the mode, its reviewer and an abort signal. */
export interface ClassificationRequest {
  readonly call: PreparedCall;
  readonly mode: PermissionMode;
  readonly classifier?: PermissionClassifier;
  readonly signal?: AbortSignal;
  /**
   * The session's prompt facts (`DecideContext.cannotPrompt`, `denyInsteadOfPrompting`): a review
   * that ends in a prompt is denied where nobody may answer it.
   */
  readonly prompts?: PromptFacts;
}

/**
 * The decision once the host's reviewer has answered a `classify`. Any other decision is returned
 * unchanged and the reviewer is not asked. With no reviewer there is no verdict, and no verdict
 * denies the action (permission-modes.md, "When auto mode falls back").
 */
export const settleClassification = (
  decision: Decision,
  request: ClassificationRequest,
): Promise<Decision> => call.async('settleClassification', decision, request);
