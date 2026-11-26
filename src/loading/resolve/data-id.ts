import { coreModule } from '../../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/data_id.rs`, docs/RUST.md). */
const call = coreModule('loading/resolve/data-id');

/**
 * The `<id>` of `~/.claude/plugins/data/<id>/` (plugins/manifest-reference.md, "Environment
 * variables"): the plugin identifier with every character other than a letter, digit, `_` or `-`
 * replaced by `-`. A character is a code point, so one outside the basic plane becomes one `-`.
 */
export const pluginDataId = (identifier: string): string => call('pluginDataId', identifier);
