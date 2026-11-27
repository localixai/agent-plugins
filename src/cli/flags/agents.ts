/**
 * `--agent` and `--agents` (cli-reference.md, "CLI flags"; sub-agents.md, "Choose the subagent
 * scope" and "Invoke subagents explicitly"; errors.md, "Invalid `--agents` configuration"). `--agent`
 * names the session's agent and "overrides the `agent` setting". `--agents` is JSON of definitions
 * keyed by name; "with `--print`, the value can instead be the path to a JSON file"; Claude Code
 * "validates the value at startup and exits on an invalid one", running the checks "in order and
 * stops at the first one that fails": the JSON, the definitions, the names. `--safe-mode` and
 * `CLAUDE_CODE_SAFE_MODE` "ignore `--agents` entirely"; with `--resume` or `--continue` "an inline
 * JSON value isn't checked and the session starts; a value read from a file is checked on every
 * launch". The refusal wording is errors.md's; what the docs leave open is in
 * spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/agents.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Contribution, FlagContext } from './types.ts';

const call = coreModule('cli/flags/agents');

export const agent = (name: string): Contribution => call('agent', name);

/** "Specify an agent for the current session (overrides the `agent` setting)": the flag wins when both name one. */
export const sessionAgent = (
  flag: string | undefined,
  setting: string | undefined,
): string | undefined => call('sessionAgent', flag, setting);

export const agents = (value: string, context: FlagContext): Contribution =>
  call('agents', value, context);
