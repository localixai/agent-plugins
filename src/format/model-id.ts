/**
 * Model IDs: what a model ID names.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/model_id.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/model-id');

/**
 * Whether a model ID names a model name, family alias or version prefix: the name is the whole ID,
 * a prefix of it, or starts one of its dash-separated parts (`opus` in `claude-opus-5-5` and in
 * `my-gateway/claude-opus-5-5`). One reading for `availableModels` entries ("`opus` allows any opus
 * version", "a custom ID that embeds a family name ... counts as a specific entry for that family")
 * and for the family of a model ID (model-config.md, "Restrict model selection", "Add a custom
 * model option"). Judgment where the docs give only examples (spec/judgments.md).
 */
export const modelIdNames = (model: string, name: string): boolean =>
  call('modelIdNames', model, name);
