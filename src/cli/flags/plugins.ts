/**
 * `--plugin-dir` and `--plugin-url` (cli-reference.md, "CLI flags"; plugins/cli-reference.md,
 * "Flags that load a plugin for one session"): plugins for this session only, of the `inline` origin
 * (`<name>@inline`, `--json` scope `session`). The kit maps the flags to requests; the host reads the
 * folder (a folder that is no plugin holds plugins: `readPluginDirectory`), fetches the archive and
 * names each plugin from its manifest. Which same-named copy loads is the install stream's
 * (`copyThatLoads`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/plugins.rs`, docs/RUST.md).
 */
import type { PluginDirectoryReading } from '../../loading/plugin-folder.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Contribution, FlagContext, SessionPluginRequest } from './types.ts';

const call = coreModule('cli/flags/plugins');

/** "Each flag takes one path. Repeat the flag for more paths": a path is never split, even with a space in it. */
export const pluginDir = (paths: readonly string[], context: FlagContext): Contribution =>
  call('pluginDir', paths, context);

/** "Repeat the flag for multiple plugins, or pass space-separated URLs in a single quoted value." */
export const pluginUrl = (values: readonly string[], context: FlagContext): Contribution =>
  call('pluginUrl', values, context);

/**
 * The folders to load for one request, given what `readPluginDirectory` read in the directory: an
 * archive or a URL is one plugin; a directory is itself when it is a plugin, else each immediate
 * subfolder that holds a `.claude-plugin/plugin.json`. Only the fields it reads cross to the core.
 */
export const pluginFoldersOf = (
  request: SessionPluginRequest,
  reading: PluginDirectoryReading,
  hostVersion?: string,
): readonly string[] =>
  call(
    'pluginFoldersOf',
    { source: request.source, location: request.location, origin: request.origin },
    { isFolder: reading.isFolder, plugins: reading.plugins },
    hostVersion,
  );
