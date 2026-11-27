/**
 * A fixed location of a core package (core §6): absent is no error (§6.2); present but of the wrong
 * filesystem kind, outside the plugin root (§4.1, boundary 2) or not checkable makes that component
 * type invalid while the others load. Containment is `locate`'s, the one owner of it in the kit.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/locations.rs`, docs/RUST.md).
 */
import type { LocateContext } from '../../format/locate.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/core/locations');

export const coreLocate = (files: PluginFiles, signal?: AbortSignal): LocateContext =>
  call('coreLocate', files, signal);
