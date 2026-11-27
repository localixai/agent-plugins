/**
 * `--add-dir` (cli-reference.md, "CLI flags"; permissions.md, "Additional directories grant file
 * access, not configuration"): extra working directories for reading and editing files. Each path
 * "exists as a directory" or startup fails; "most network paths" cannot be added
 * ("... is a network path, which cannot be added as a working directory", errors.md). The
 * configuration a flag-added directory also loads is a fixed list, and the modes that remove a kind
 * remove it here too: skills, commands and subagents "load through the `project` setting source, so
 * they don't load when you exclude" it, `--bare` "skips the commands and subagents among them" but
 * keeps the skills, and `CLAUDE.local.md` "additionally requires the `local` setting source". What
 * loads is derived from the plan once every flag is combined (`session-flags.ts`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/add_dir.rs`, docs/RUST.md).
 */
import type { PolicyScope } from '../../settings/layers.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Contribution, Customization, FlagContext } from './types.ts';

const call = coreModule('cli/flags/add-dir');

/** The configuration types loaded from an `--add-dir` directory. */
export const ADD_DIR_CONFIGURATION: readonly [
  'skills',
  'commands',
  'agents',
  'settings:enabledPlugins',
  'settings:extraKnownMarketplaces',
  'claude-md',
  'claude-local-md',
] = call('ADD_DIR_CONFIGURATION');
export type AddDirConfiguration = (typeof ADD_DIR_CONFIGURATION)[number];

/** What decides the load: the combined plan of the flags and the one variable the docs name. */
export interface LoadFacts {
  readonly bare: boolean;
  readonly claudeMdEnabled: boolean;
  /** The customizations the modes skip (`SessionPlan.skipped`). */
  readonly skipped: readonly Customization[];
  /** The settings scopes that load; absent means every one (`SessionPlan.settingScopes`). */
  readonly settingScopes?: readonly PolicyScope[];
}

/** The kinds a flag-added directory loads under the plan. */
export const addDirLoads = (facts: LoadFacts): readonly AddDirConfiguration[] =>
  call('addDirLoads', facts);

/** `--add-dir <paths…>`: the directories added, or the startup failure. */
export const addDir = (paths: readonly string[], context: FlagContext): Contribution =>
  call('addDir', paths, context);
