import type { AgentDefinition, AgentMcpServerSpec } from '../protocol/official.ts';
import type { AgentSource } from '../format/agent-record.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * A teammate spawned from an agent definition (agent-teams.md, "Use subagent definitions for
 * teammates"; spec §5.12.6): which parts of the definition apply, by display mode, and whether a
 * stopped teammate brought back gets its definition again. Pure: spawning, panes and the prompt are
 * the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/teammate.rs`, docs/RUST.md).
 */
const call = coreModule('agents/teammate');

/** In process (inside the lead's session, the default), or in its own split pane. */
export type TeammateDisplay = 'in-process' | 'split-pane';

export interface TeammateInput {
  /** The definition the teammate is spawned from (`AgentRecord.definition`). */
  readonly definition: AgentDefinition;
  readonly display: TeammateDisplay;
  /** The model the spawn request names, which wins over the definition's. */
  readonly requestedModel?: string;
  /** The session has the Task tools. */
  readonly taskTools: boolean;
  /** Where the definition came from: a plugin agent's `mcpServers` stays ignored (§5.6.1). */
  readonly source: AgentSource;
}

/** What the definition gives the teammate; a field left out is the teammate's default. */
export interface TeammateSetup {
  /** The tools from the definition's `tools`, with the team tools in process; absent without a list. */
  readonly tools?: readonly string[];
  /** Removed from the teammate's tools (in process only), never the team tools. */
  readonly disallowedTools?: readonly string[];
  readonly model?: string;
  readonly effort?: AgentDefinition['effort'];
  readonly autoCompactWindow?: number;
  /** The body: appended to the default system prompt in process, in place of it in a split pane. */
  readonly prompt: { readonly placement: 'append' | 'replace'; readonly text: string };
  /** Applied in a split pane only, and never a plugin agent's. */
  readonly mcpServers?: readonly AgentMcpServerSpec[];
}

/**
 * The parts of a definition a teammate gets. `skills` is never applied: a teammate loads skills
 * (and, in process, MCP servers) from the project and user settings.
 */
export const teammateSetup = (input: TeammateInput): TeammateSetup => call('teammateSetup', input);

/** The notice after the resume report when a stopped teammate comes back without its definition. */
export const DEFINITION_NOT_RESTORED: string = call('DEFINITION_NOT_RESTORED');

export interface TeammateRestore {
  /** The definition's source (`project` also stands for an `--add-dir` directory). */
  readonly source: AgentSource;
  /** The folder holding the agent file is itself trusted; a trusted parent does not count. */
  readonly folderTrusted: boolean;
}

/**
 * Bringing back a stopped in-process teammate: its definition is re-applied, or it returns with
 * only the team tools and `notice` follows the resume report in the sender's tool result.
 */
export const restoreTeammate = (
  input: TeammateRestore,
): {
  readonly reapplied: boolean;
  readonly notice?: string;
} => call('restoreTeammate', input);
