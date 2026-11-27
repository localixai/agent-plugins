/**
 * The keys of a plugin entry and the type of each (plugins/marketplace-reference.md, "Plugin
 * entries"): the entry's own fields, and every `plugin.json` field ("an entry also accepts every
 * `plugin.json` field") except the directory listing fields, which "claude plugin validate reports
 * ... as an unknown field" (plugins/manifest-reference.md, "Directory listing fields"). The shared
 * fields reuse the manifest's schemas, so an entry and a plugin.json cannot disagree on a type.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/entry_schema.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/entry-schema');

/** Every key an entry knows: any other is reported as an unknown field. */
export const ENTRY_KEYS: readonly string[] = call('ENTRY_KEYS');
