/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/responses/agent_response.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import type { AgentOutput } from '../../../protocol/official-tools.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/responses/agent-response');

/**
 * The two members of a foreground or background Agent call's `tool_response` that hooks.md ("Agent")
 * documents. Each type is a `Pick` of the SDK's `AgentOutput` member, so a field the SDK renames or
 * retypes breaks the typecheck here and not in a hook. Fields the table does not list (`agentType`,
 * `toolStats`, a block's `citations`, the SDK's remote launch) are left out (spec/judgments.md).
 */
type Completed = Extract<AgentOutput, { status: 'completed' }>;
type Launched = Extract<AgentOutput, { status: 'async_launched' }>;

/** A final text block of the subagent, or the note that stands in for a `SubagentHandback` report. */
export type AgentTextBlock = Readonly<Pick<Completed['content'][number], 'type' | 'text'>>;

/** "Per-type token breakdown of the final API request": the four keys the table names. */
export type AgentUsage = Readonly<
  Pick<
    Completed['usage'],
    'input_tokens' | 'output_tokens' | 'cache_creation_input_tokens' | 'cache_read_input_tokens'
  >
>;

/** `status: "completed"`: a foreground subagent, with its run telemetry. */
export type CompletedAgentResponse = Readonly<
  Pick<
    Completed,
    | 'status'
    | 'agentId'
    | 'resolvedModel'
    | 'modelsUsed'
    | 'totalTokens'
    | 'totalDurationMs'
    | 'totalToolUseCount'
  >
> & {
  readonly content: readonly AgentTextBlock[];
  readonly usage: AgentUsage;
};

/** `status: "async_launched"`: a background subagent. It has no usage fields. */
export type LaunchedAgentResponse = Readonly<
  Pick<
    Launched,
    'status' | 'agentId' | 'description' | 'prompt' | 'outputFile' | 'resolvedModel' | 'modelsUsed'
  >
>;

export type AgentResponse = CompletedAgentResponse | LaunchedAgentResponse;

export interface AgentResponseParse {
  /** `undefined` when the response could not be read as either member. */
  readonly response: AgentResponse | undefined;
  /** Each problem, at `tool_response.<field>`. */
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads the `tool_response` a PostToolUse hook receives for an Agent call (hooks.md, "Agent"): the
 * `completed` response of a foreground subagent or the `async_launched` response of a background
 * one. A wrong type or a missing field is an issue and nothing throws; `resolvedModel` is accepted
 * whatever the request's `model` said ("which may differ from the requested model").
 */
export const parseAgentResponse = (raw: unknown): AgentResponseParse =>
  call('parseAgentResponse', raw);
