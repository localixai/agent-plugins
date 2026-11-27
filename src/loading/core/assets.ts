/**
 * The package's files that are data, not components: icon candidates under `assets/` (core §4.3.1)
 * and the extension directories of the namespaces the host implements (core §8.2). A candidate or a
 * directory that resolves outside the plugin root is left out, its access denied (core §4.1,
 * boundary 6); neither ever invalidates the package. `assets/` is never an extension directory (§8).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/assets.rs`, docs/RUST.md).
 */
import type { LocateContext } from '../../format/locate.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/core/assets');

/** The usable-looking icon candidates, PNG first. */
export const coreIcons = (
  context: LocateContext,
  formats: readonly string[],
): Promise<readonly string[]> => call.async('coreIcons', context, formats);

/** The top-level directories named exactly for an implemented namespace, in name order. */
export const coreExtensionDirectories = (
  context: LocateContext,
  namespaces: readonly string[],
): Promise<readonly string[]> => call.async('coreExtensionDirectories', context, namespaces);
