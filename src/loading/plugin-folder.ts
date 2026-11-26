import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/plugin_folder.rs`, docs/RUST.md). */
const call = coreModule('loading/plugin-folder');

/**
 * What a folder passed as `--plugin-dir` is (plugins/create.md, "Load a folder of plugins"): "If the
 * folder has no `.claude-plugin/` directory and no plugin components at its top level, Claude Code
 * treats it as a folder of plugins", whose children that have a manifest load as plugins of their
 * own; the folder itself then loads nothing. A host loads a folder of plugins by classifying each
 * child, then calls `loadPlugin` on those that are plugins.
 */
export type PluginFolderKind = 'plugin' | 'folder-of-plugins';

/**
 * Judgment (spec/judgments.md, oracle 2.1.319): the docs name the rule, not its edges. A
 * `.claude-plugin/` folder marks a plugin unless it holds a `marketplace.json` and no `plugin.json`
 * (the docs' marketplace-beside-plugins case); `.mcp.json` marks one unless it is zero bytes;
 * `settings.json`, `bin/`, `CLAUDE.md` and a lower-case `skill.md` mark nothing.
 */
export const classifyPluginFolder = (input: {
  readonly files: PluginFiles;
  readonly signal?: AbortSignal | undefined;
}): Promise<PluginFolderKind> => call.async('classifyPluginFolder', input.files, input.signal);

export interface PluginDirectoryReading {
  /** The directory is a folder that holds plugins, not a plugin itself. */
  readonly isFolder: boolean;
  /** The immediate subfolders that load, as separate plugins, in name order; none for a plugin. */
  readonly plugins: readonly string[];
}

/**
 * A folder passed as `--plugin-dir` and the plugins it loads: "Each immediate subfolder that has a
 * `.claude-plugin/plugin.json` manifest then loads as a separate plugin. Everything else in the
 * folder is skipped without an error, including a subfolder that has no manifest."
 */
export const readPluginDirectory = (
  files: PluginFiles,
  { signal }: { readonly signal?: AbortSignal | undefined } = {},
): Promise<PluginDirectoryReading> => call.async('readPluginDirectory', files, signal);
