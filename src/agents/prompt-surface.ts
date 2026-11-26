/**
 * What decides where a subagent's permission prompts appear (tools-reference.md, "Agent tool
 * behavior").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/prompt_surface.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/prompt-surface');

/** Where a subagent's permission prompts appear (tools-reference.md, "Agent tool behavior"). */
export interface SubagentPromptSurface {
  /** `call`: the same prompts as the main conversation, "at the moment each tool call happens"; `main-session`: surfaced in the main session. */
  readonly appearsIn: 'call' | 'main-session';
  /** "The prompt names which subagent is asking." */
  readonly namesSubagent: boolean;
  /** "Pressing Esc denies that one tool call without stopping the subagent." */
  readonly escDeniesOneCall: boolean;
}

/**
 * "Launching the subagent doesn't itself prompt for permission": its own tool calls are checked
 * against the permission rules as it runs, and where the prompt shows depends on whether it runs in
 * the foreground or the background. Judgment (spec/judgments.md): the foreground prompt does not name
 * the subagent, because the docs say it is the one the main conversation shows.
 */
export const permissionSubagentPromptSurface = (background: boolean): SubagentPromptSurface =>
  call('permissionSubagentPromptSurface', background);
