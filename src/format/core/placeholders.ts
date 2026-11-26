/**
 * The two plugin variables of the core (core §9): `${PLUGIN_ROOT}`, the filesystem-resolved plugin
 * root, and `${PLUGIN_DATA}`, the client-managed data directory of the installed instance. Expansion
 * is one non-recursive textual pass over every exact occurrence of either placeholder: text a
 * replacement introduces is never scanned again, and any other `${...}` stays literal (core §9.2).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/placeholders.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

/** The two directories, absolute and `/`-spelled, as the host resolved them. */
export interface CorePaths {
  readonly root: string;
  readonly data: string;
}

const call = coreModule('format/core/placeholders');

export const CORE_VARIABLES: readonly ['PLUGIN_ROOT', 'PLUGIN_DATA'] = call('CORE_VARIABLES');
export type CoreVariable = (typeof CORE_VARIABLES)[number];

/** `text` with both placeholders replaced in one pass (core §9.2). */
export const expandPlaceholders = (text: string, paths: CorePaths): string =>
  call('expandPlaceholders', text, paths);

/** A resolved `cwd`: under which directory, the path below it, and the absolute path. */
export interface ResolvedCwd {
  readonly base: CoreVariable;
  /** `/`-separated, no `.` or `..`; `.` for the base itself. */
  readonly below: string;
  readonly path: string;
}

/**
 * Expands, then resolves a `cwd` (core §7.2.1: "Clients MUST expand placeholders before resolving
 * `cwd`"), or says why the server entry is invalid: another form, or a path that climbs out of its
 * base. The containment checked here is textual; whether a link below the base leads out is the
 * loader's question for the plugin root (`loadCorePackage`) and the host's for the data directory.
 */
export const resolveCwd = (
  cwd: string,
  paths: CorePaths,
): { readonly cwd: ResolvedCwd } | { readonly problem: string } => call('resolveCwd', cwd, paths);
