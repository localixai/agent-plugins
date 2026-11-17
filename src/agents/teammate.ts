import type { AgentDefinition, AgentMcpServerSpec } from '../protocol/official.ts';
import type { AgentSource } from '../format/agent-record.ts';

/**
 * A teammate spawned from an agent definition (agent-teams.md, "Use subagent definitions for
 * teammates"; spec §5.12.6): which parts of the definition apply, by display mode, and whether a
 * stopped teammate brought back gets its definition again. Pure: spawning, panes and the prompt are
 * the host's.
 */

/** In process (inside the lead's session, the default), or in its own split pane. */
export type TeammateDisplay = 'in-process' | 'split-pane';

/** The tools every in-process teammate gets beside its list: `SendMessage`, then the Task tools. */
const TEAM_TOOL = 'SendMessage';
const TASK_TOOLS = ['TaskCreate', 'TaskGet', 'TaskList', 'TaskUpdate'] as const;

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

const teamTools = (taskTools: boolean): readonly string[] =>
  taskTools ? [TEAM_TOOL, ...TASK_TOOLS] : [TEAM_TOOL];

/** A plugin agent's `mcpServers` is never applied (§5.6.1). */
const PLUGIN_MCP_IGNORED: { readonly [S in AgentSource]: boolean } = {
  managed: false,
  flag: false,
  project: false,
  user: false,
  plugin: true,
};

/** What each display applies beyond `model` (agent-teams.md's table). */
const BY_DISPLAY: {
  readonly [D in TeammateDisplay]: (input: TeammateInput) => Omit<TeammateSetup, 'model'>;
} = {
  'in-process': ({ definition, taskTools }) => {
    const essential = new Set(teamTools(taskTools));
    const tools = definition.tools && [...definition.tools, ...teamTools(taskTools)];
    const removed = definition.disallowedTools?.filter((tool) => !essential.has(tool));
    const { effort, autoCompactWindow } = definition;
    return {
      ...(tools === undefined ? {} : { tools: [...new Set(tools)] }),
      ...(removed === undefined ? {} : { disallowedTools: removed }),
      ...(effort === undefined ? {} : { effort }),
      ...(autoCompactWindow === undefined ? {} : { autoCompactWindow }),
      prompt: { placement: 'append', text: definition.prompt },
    };
  },
  'split-pane': ({ definition, source }) => {
    const { tools, mcpServers } = definition;
    const servers = PLUGIN_MCP_IGNORED[source] ? undefined : mcpServers;
    return {
      ...(tools === undefined ? {} : { tools }),
      ...(servers === undefined ? {} : { mcpServers: servers }),
      prompt: { placement: 'replace', text: definition.prompt },
    };
  },
};

/**
 * The parts of a definition a teammate gets. `skills` is never applied: a teammate loads skills
 * (and, in process, MCP servers) from the project and user settings.
 */
export function teammateSetup(input: TeammateInput): TeammateSetup {
  const model = input.requestedModel ?? input.definition.model;
  const applied = BY_DISPLAY[input.display](input);
  return { ...applied, ...(model === undefined ? {} : { model }) };
}

/** The notice after the resume report when a stopped teammate comes back without its definition. */
export const DEFINITION_NOT_RESTORED =
  "Its agent definition was not restored: the folder its definition file came from is not trusted (source: projectSettings), so the teammate is running with the team-essential tools and no custom instructions. To restore it, the user needs to run Claude Code in that folder once and accept the trust dialog (the --debug log names the folder); do not change trust settings on the user's behalf.";

/** Whose definition is re-applied only from a trusted folder: the `.claude/agents/` of the project or an `--add-dir` directory. */
const NEEDS_TRUSTED_FOLDER: { readonly [S in AgentSource]: boolean } = {
  managed: false,
  flag: false,
  project: true,
  user: false,
  plugin: false,
};

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
export function restoreTeammate(input: TeammateRestore): {
  readonly reapplied: boolean;
  readonly notice?: string;
} {
  const reapplied = !NEEDS_TRUSTED_FOLDER[input.source] || input.folderTrusted;
  return reapplied ? { reapplied } : { reapplied, notice: DEFINITION_NOT_RESTORED };
}
