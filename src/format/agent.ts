/**
 * One agent file read as Claude Code would load it (sub-agents.md, "Frontmatter reference"): a
 * project, user or managed file, or a plugin's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { AgentOrigin, AgentReadResult } from './agent-record.ts';

const call = coreModule('format/agent');

/**
 * Read one agent file (sub-agents.md, "Frontmatter reference"): the Markdown body is the system
 * prompt, `name` and `description` are required. Never throws; a file Claude Code would skip is
 * `skipped`, with the reason it names.
 */
export const readAgent = (text: string, origin: AgentOrigin): AgentReadResult =>
  call('readAgent', text, origin);
