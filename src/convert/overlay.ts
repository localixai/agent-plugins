import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ConvertedFile } from './types.ts';

/**
 * A plugin folder as it would be once a conversion's files are written (Agent Plugins Extended
 * §18.4.1 item 3): reads of a written path answer with its text, the folders above it exist, and
 * every other read goes to the folder itself. Nothing is written; it lets the target's own loader
 * and validator judge the output before a host writes it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/overlay.rs`, docs/RUST.md).
 */
const call = coreModule('convert/overlay');

export const overlayFiles = (base: PluginFiles, written: readonly ConvertedFile[]): PluginFiles =>
  call('overlayFiles', base, written);
