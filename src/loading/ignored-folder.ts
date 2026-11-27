import type { PluginIssue } from '../format/issues.ts';
import type { ComponentKind } from '../format/layout.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { DiscoverContext } from './discovered.ts';

/**
 * The warning for a default folder that exists and that a replacing manifest key leaves unscanned
 * (plugins/manifest-reference.md, "How each key combines with its default location"): "To avoid the
 * warning, set the key to a path inside that folder", and listing the folder itself keeps it. Only a
 * kind whose key replaces its default has a folder to ignore.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/ignored_folder.rs`, docs/RUST.md).
 */
const call = coreModule('loading/ignored-folder');

export const ignoredFolderIssues = (
  kind: ComponentKind,
  declaredPaths: readonly string[],
  context: DiscoverContext,
): Promise<PluginIssue[]> => call.async('ignoredFolderIssues', kind, declaredPaths, context);
