/**
 * `agent` and `disableAgentView` (settings-reference.md, `agent`, `disableAgentView`). `--agent` and
 * `--teammate-mode` for one session are `SESSION_OVERRIDES` rows; `CLAUDE_CODE_DISABLE_AGENT_VIEW`
 * is paired with its key here.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/agent_keys.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/general/agent-keys');

export const readAgentSetting = (view: PolicyView): string | undefined =>
  call('readAgentSetting', view);

export interface AgentViewDecision {
  readonly disabled: boolean;
  /** Everything that turned it off: any one of them keeps it off. */
  readonly offBy: readonly ('disableAgentView' | 'env')[];
}

/**
 * Whether background agents and agent view are off: "whichever of the two turns it off, the other
 * can't turn it back on". The variable takes the usual on values (env-vars.md: "`1`, `true`, `yes`,
 * or `on` ... in any casing"); an off value, an empty one or an unlisted one leaves it unset.
 */
export const agentViewDisabled = (
  view: PolicyView,
  env: Readonly<Record<string, string | undefined>>,
): AgentViewDecision => call('agentViewDisabled', view, env);
