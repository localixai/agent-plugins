import { coreModule } from '../wasm/bridge.ts';
import type { UsageError } from './result.ts';
import type { FlagFacts, GivenFlags, SessionPlan } from './flags/types.ts';

/**
 * The flags of one `claude` run as a plan (cli-reference.md, "CLI flags"; plugins/cli-reference.md,
 * "Flags that load a plugin for one session"). Each flag is a strategy over `FlagName`
 * (`flags/registry.ts`); the plan is what they add up to, handed to the modules that own each
 * behaviour: settings layers, session plugins, the tool filter, the permission mode, the agents,
 * the channel verdict, the MCP sources. Pure over what the host parsed and read. A plan with
 * `refusals` is a run that exits `1` without starting.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/session_flags.rs`, docs/RUST.md).
 */

const call = coreModule('cli/session-flags');

/** What the flags a run was given add up to, in the order of `FLAG_NAMES`. */
export const planSessionFlags = (given: GivenFlags, facts: FlagFacts): SessionPlan =>
  call('planSessionFlags', given, facts);

/** What the run ends with when the plan refuses it: exit `1`, the reasons on stderr, no result line. */
export const startupRefusal = (plan: SessionPlan): UsageError | undefined =>
  call('startupRefusal', plan);
