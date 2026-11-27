import { coreModule } from '../wasm/bridge.ts';
import type { DependencyIssue } from './dependencies.ts';
import type { CopyLookup, GraphFacts } from './dependency-graph.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/plugin_dir_dependencies.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/plugin-dir-dependencies');

/** A plugin a session loads with `--plugin-dir`: its name and its manifest's `dependencies`. */
export interface PluginDirPlugin {
  readonly name: string;
  readonly dependencies?: readonly unknown[] | undefined;
}

/** A `--plugin-dir` plugin that does not load, and the dependency issue that stops it. */
export interface UnloadedPlugin {
  readonly name: string;
  readonly issue: DependencyIssue;
}

/**
 * The `--plugin-dir` plugins of a session that do not load for a dependency (plugins/dependencies.md,
 * "Test a plugin and its dependency locally"): the session's other plugins are the local copies that
 * meet an entry, and a plugin "stops loading whenever the local copy is disabled or absent".
 * `facts` adds what is installed (an installed copy wins, `findDependencyCopy`). Judgments
 * (spec/judgments.md): oracle 2.1.319, a plugin that does not load is no copy, so a plugin that needs
 * it stops loading too; an entry `normalizeDependency` refuses is left to the manifest's validation.
 */
export const pluginDirDependencyIssues = (
  plugins: readonly PluginDirPlugin[],
  facts?: GraphFacts,
  lookup?: CopyLookup,
): readonly UnloadedPlugin[] => call('pluginDirDependencyIssues', plugins, facts, lookup);
