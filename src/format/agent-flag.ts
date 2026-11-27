/**
 * The agents of the `--agents` flag (sub-agents.md, "Choose the subagent scope"): a JSON object whose
 * top-level keys are agent names and whose values are definitions. The host reads and parses the flag
 * (or the file `-p` takes); this reads the parsed value the way Claude Code reads a definition:
 * `prompt` is the system prompt, the other keys are the frontmatter fields of a file, and `color` and
 * `experimental` "aren't accepted here and are ignored rather than rejected".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_flag.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { AgentRecord } from './agent-record.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/agent-flag');

export interface FlagAgentsRead {
  readonly agents: readonly AgentRecord[];
  /** Errors name `--agents` or `--agents.<name>`; a refused definition is not in `agents`. */
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads the parsed `--agents` value. An invalid value loads no agent at all: Claude Code exits with
 * code 1 and prints the first failing check only (§5.5.1), so `agents` is empty while any error is
 * reported, and the host refuses to start the session. Judgment (spec/judgments.md): the
 * docs do not say which keys a definition must
 * have, so `description` and `prompt` are required (they are the two fields every subagent has).
 */
export const readFlagAgents = (raw: unknown): FlagAgentsRead => call('readFlagAgents', raw);
