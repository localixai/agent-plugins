/**
 * Reads the text of what discovery found for a kind that holds Markdown. This is the seam the stream
 * that owns the component's content replaces with its own parser (skill, command, agent, output
 * style); until then the text is kept as written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/readers_text.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { DiscoveredFile } from './discovered.ts';
import type { PluginTextFile } from './plugin-model.ts';
import type { ComponentRead, ReaderContext, ReadKind } from './reader-types.ts';

const call = coreModule('loading/readers-text');

export const textFiles =
  (kind: ReadKind, find: (context: ReaderContext) => readonly DiscoveredFile[]) =>
  (context: ReaderContext): Promise<ComponentRead<readonly PluginTextFile[]>> =>
    call.async('textFiles', kind, find(context), context);
