/**
 * What makes an agent's name usable and how a plugin agent is named (sub-agents.md, "Frontmatter
 * reference", "Subagent files Claude Code skips"; plugins/components.md, "Organize agents in
 * subfolders").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_name.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { AgentRecord } from './agent-record.ts';

const call = coreModule('format/agent-name');

/**
 * Why a name cannot be used by a project, user, managed or `--add-dir` agent, or undefined when it
 * can (sub-agents.md, "Subagent files Claude Code skips": a name that "starts with `-`, contains
 * `:`, or is longer than 256 characters"). Judgment: length counts UTF-16 code units (spec/judgments.md).
 */
export const agentNameProblem = (name: string): string | undefined =>
  call('agentNameProblem', name);

export const validAgentName = (name: string): boolean => call('validAgentName', name);

/**
 * The last segment of a path without its `.md`: what a plugin agent is named without a `name`.
 * Judgment (spec/judgments.md): a segment that is nothing but `.md` keeps its whole text, so a
 * name is never empty.
 */
export const agentFileName = (path: string): string => call('agentFileName', path);

/**
 * The name a plugin agent is registered under (plugins/components.md, "Organize agents in
 * subfolders"): the plugin name, each subfolder of `relativePath` and `name`, joined by colons.
 * `name` takes the place of the file name only, so `review/security.md` with `name: audit` is
 * `my-plugin:review:audit`. A file the manifest lists has no `relativePath`: no subfolder names.
 */
export const scopedAgentName = (
  plugin: string,
  relativePath: string | undefined,
  name: string,
): string => call('scopedAgentName', plugin, relativePath, name);

/** The `agent_type` hooks receive for this agent, and the value a hook matcher is matched against. */
export const hookAgentType = (agent: Pick<AgentRecord, 'name' | 'scopedName' | 'source'>): string =>
  call('hookAgentType', { name: agent.name, scopedName: agent.scopedName, source: agent.source });
