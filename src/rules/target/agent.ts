import { coreModule } from '../../wasm/bridge.ts';
import type { SpecifierMatcher } from '../matchers.ts';

/**
 * `Agent(AgentName)` (permissions.md, "Agent (subagents)"): the rule's text is the subagent's name
 * and is compared with the call's `subagent_type`. The comparison is exact and case-sensitive. A
 * fork is the call whose `subagent_type` is `fork` (the SDK's `AgentInput` names it so), which
 * `Agent(fork)` stops (sub-agents.md, "Turn fork mode off"). A call that leaves `subagent_type` out
 * is `general-purpose` for a deny or ask rule and no name for an allow rule (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/target/agent.rs`, docs/RUST.md).
 */
const call = coreModule('rules/target/agent');

export const matchAgentRule: SpecifierMatcher = (rule, prepared, ctx) =>
  call('matchAgentRule', rule, prepared, ctx);
