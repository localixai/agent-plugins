/**
 * The permission mode a subagent runs in (sub-agents.md, "Permission modes"): "The main
 * conversation's permission mode decides whether Claude Code uses the value you set". Unset, the
 * subagent inherits the main conversation's mode. A value that is no mode counts as unset.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/subagent_permission.rs`, docs/RUST.md).
 */
import type { PermissionMode } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/subagent-permission');

/** What decides the subagent's mode beyond its own `permissionMode`. */
export interface SubagentPermissionInput {
  /** The main conversation's current mode. */
  readonly main: PermissionMode;
  /**
   * The subagent's `permissionMode` as text (a `PermissionModeInput`; "manual" is `default`);
   * `undefined` when it sets none. Raw text from a host that is no mode counts as unset.
   */
  readonly set?: string | undefined;
  /** Auto mode is available to the subagent: no `disableAutoMode`, a model that supports it. */
  readonly autoAvailable: boolean;
}

/** How the mode came about: `inherited` (nothing set), `set` (its own applies), `main` (the main's held). */
export type SubagentModeBasis = 'inherited' | 'set' | 'main';

export interface SubagentPermission {
  readonly mode: PermissionMode;
  /**
   * An action the mode would ask about is denied instead: `dontAsk`, and a subagent that
   * "stays in plan mode" while it sets `dontAsk`.
   */
  readonly denyInsteadOfAsk: boolean;
  readonly basis: SubagentModeBasis;
}

export const resolveSubagentPermissionMode = (input: SubagentPermissionInput): SubagentPermission =>
  call('resolveSubagentPermissionMode', input);
