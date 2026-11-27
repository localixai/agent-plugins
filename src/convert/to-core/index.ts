import type { Plugin } from '../../loading/plugin-model.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Conversion, ConvertOptions } from '../types.ts';

const call = coreModule('convert/to-core/index');

export interface ClaudeCodeToCoreOptions extends ConvertOptions {
  /**
   * The plugin's `hooks/hooks.json` declares `modules` (a mod), which `Plugin` does not carry; it is
   * reported as not converted. `convertPluginPackage` reads it.
   */
  readonly mods?: boolean;
}

/**
 * A Claude Code plugin as a core package (Agent Plugins Extended §18.4.3), pure over the loaded
 * plugin: `plugin.json`, `mcp.json` and `hooks.json` to write beside it, making a dual package
 * (§18.4.1 item 4), and a note for everything §18.4.5 lists as having no core counterpart. A name
 * core §5.5 refuses stops the conversion.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/index.rs`, docs/RUST.md).
 */
export const claudeCodeToCore = (
  source: Plugin,
  options: ClaudeCodeToCoreOptions = {},
): Conversion => call('claudeCodeToCore', source, options);
