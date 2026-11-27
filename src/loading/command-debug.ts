/**
 * The debug-log line for a manifest `commands` directory that holds no command.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/command_debug.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PluginTextFile } from './plugin-model.ts';
import type { ReaderContext } from './reader-types.ts';

const call = coreModule('loading/command-debug');

/**
 * troubleshooting.md: this diagnostic belongs in the debug log, not the Plugin Errors tab, with the
 * directory and the advice after the colon. The path is the manifest's, as written: Claude Code
 * prints the absolute one, which is the host's (`Locations.pluginRoot`), not the loader's.
 */
export const commandDirectoryDebug = (
  context: ReaderContext,
  commands: readonly PluginTextFile[],
): Promise<readonly string[]> => call.async('commandDirectoryDebug', context, commands);
