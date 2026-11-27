import type { PolicyView } from '../view.ts';
import type { WorkflowsContext, WorkflowsDecision } from './enablement.ts';
import type { KeywordVerdict, PromptInput } from './keyword.ts';
import type { WorkflowSizeContext, WorkflowSizeDecision } from './size.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/workflows/policy');

/** What the host tells the workflow keys besides the layers. */
export type WorkflowPolicyContext = WorkflowsContext & WorkflowSizeContext;

/**
 * The workflow keys of the settings as one object over a view (docs/API.md, "Settings").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/workflows/policy.rs`, docs/RUST.md).
 */
export interface WorkflowSettingsPolicy {
  enablement(): WorkflowsDecision;
  readonly workflowsEnabled: boolean;
  keyword(prompt: PromptInput): KeywordVerdict;
  size(): WorkflowSizeDecision;
}

export const createWorkflowSettingsPolicy = (
  view: PolicyView,
  context: WorkflowPolicyContext,
): WorkflowSettingsPolicy =>
  call('createWorkflowSettingsPolicy', view, {
    env: context.env,
    plan: context.plan,
    orgWorkflowsOff: context.orgWorkflowsOff,
    configChoice: context.configChoice,
    ultracode: context.ultracode,
  });
