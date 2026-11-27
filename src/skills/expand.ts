/**
 * Expanding a skill or a command into what the model receives and what the host must apply
 * (skills.md, "Inject dynamic context", "Available string substitutions", "Pass arguments to
 * skills", "Pre-approve tools for a skill", "Run skills in a subagent"). The order is the
 * documented one: the placeholders are found in the original text, arguments are substituted, then
 * `${CLAUDE_*}`, then the commands run, and their output is never scanned again. Effects go
 * through the ports; a plugin error is an issue, never a throw.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/expand.rs`, docs/RUST.md).
 */
import type { Command, Skill } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ExpandPorts, ExpandRequest, Expansion } from './expand-types.ts';

export type { ExpandPolicy, SkillPluginSource } from './expand-policy.ts';
export type {
  ExpandPorts,
  ExpandRequest,
  ExpandSession,
  Expansion,
  SessionLimits,
} from './expand-types.ts';
export type { ShellCall, ShellPermission } from './shell-run.ts';

const call = coreModule('skills/expand');

type Source = Skill | Command;

/** Expands `skill` for one invocation. */
export const expandSkill = (
  skill: Source,
  request: ExpandRequest,
  ports: ExpandPorts,
): Promise<Expansion> => call.async('expandSkill', skill, request, ports);
