/**
 * A core package as a Claude Code plugin (Agent Plugins Extended §18.4.2), pure over the loaded
 * package: the files to write beside it, which make a dual package (§18.4.1 item 4), and a note for
 * everything not carried over (item 1). The extension data the package was loaded with is reported,
 * so a host loads it with every namespace it holds (`convertPluginPackage` does) to see all of it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_claude_code/index.rs`, docs/RUST.md).
 */
import type { CorePackage } from '../../loading/core/model.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { Conversion, ConvertOptions } from '../types.ts';

const call = coreModule('convert/to-claude-code/index');

export const coreToClaudeCode = (source: CorePackage, options: ConvertOptions = {}): Conversion =>
  call('coreToClaudeCode', source, options);
