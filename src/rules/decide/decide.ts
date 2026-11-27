/**
 * Whether a call may run, asks the user, is denied or goes to the host's classifier
 * (permissions.md, "Manage permissions", "Permission modes" and "Extend permissions with hooks";
 * permission-modes.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/decide.rs`, docs/RUST.md),
 * with the facts of a call, the modes, the rules stage, the hooks and mods it reads.
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Decision, PreparedCall } from '../types.ts';
import type { DecideContext } from './context.ts';

const call = coreModule('rules/decide/decide');

/**
 * Whether a call may run, asks the user, is denied or goes to the host's classifier
 * (permissions.md, "Manage permissions", "Permission modes" and "Extend permissions with hooks";
 * permission-modes.md). Pure over the call and the session: the host supplies the hook outcome, the
 * mod's answer and the session facts in `ctx`, and acts on the answer.
 *
 * The order: a path outside the working directories that the block keys refuse; a hook block; deny
 * rules; ask rules and hook asks; the actions no mode auto-approves (critical-path removals, user
 * interaction, network reads); protected paths; then allow rules, hook allows and read-only calls;
 * then what the mode does with the rest. A mod's answer comes last.
 *
 * The context's three functions cross beside its data: a record that holds a function would cross
 * as a port, read member by member.
 */
export const decide = (prepared: PreparedCall, ctx: DecideContext): Decision => {
  const { requiresPermission, preapprovedFetchDomain, foreignArtifact, ...data } = ctx;
  return call(
    'decide',
    prepared,
    data,
    requiresPermission,
    preapprovedFetchDomain,
    foreignArtifact,
  );
};
