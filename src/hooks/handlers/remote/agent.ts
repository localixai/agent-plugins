/**
 * An `agent` hook (hooks.md, "Agent-based hooks"): "like prompt-based hooks but with multi-turn
 * tool access": a subagent that "can use tools like Read, Grep, and Glob to investigate" and returns
 * `{ "ok": true/false }`. It has no `continueOnBlock` and no `impossible` ("don't support the
 * prompt-hook `impossible` field"). Skipping it on `PermissionRequest` is the select stage's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/agent.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('hooks/handlers/remote/agent');

/** "Tools like Read, Grep, and Glob": the read-only investigation tools the docs name (judgment). */
export const AGENT_HOOK_TOOLS: readonly string[] = call('AGENT_HOOK_TOOLS');
