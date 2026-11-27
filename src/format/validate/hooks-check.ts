import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

const call = coreModule('format/validate/hooks-check');

/**
 * The hook configuration checks of `claude plugin validate` (oracle 2.1.319), for the `hooks`
 * object of `hooks/hooks.json` and the inline one of `plugin.json`. An entry that cannot load is
 * dropped at runtime and reported as a warning, except under `PreToolUse` and `PermissionRequest`:
 * a guard that silently went missing could leave the permissions beside it unprotected, so there
 * it is an error.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/hooks_check.rs`, docs/RUST.md).
 */

/** Why a guard that cannot load is an error. */
export const GUARDED_TAIL: string = call('GUARDED_TAIL');

/** Where a hooks object sits and whether its shell commands are advised on. */
export interface HooksSite {
  /** How the oracle names it: `hooks`, or `hooks.0` for one map of a list. */
  readonly prefix: string;
  /** The unquoted-placeholder warning applies to `hooks/hooks.json` only, not to hooks written inline. */
  readonly advice: boolean;
}

/** The findings of one hooks object, event by event. */
export const hooksMapFindings = (
  map: Readonly<Record<string, unknown>>,
  site?: HooksSite,
): Finding[] => call('hooksMapFindings', map, site);
