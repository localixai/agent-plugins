/**
 * `relevance` of a plugin entry (plugins/relevance.md, "Field reference" and "Validate your
 * marketplace"): the typed model and its limits. Unknown fields under `relevance` and
 * `relevance.signals` are ignored with a warning, "because ... older clients still load a
 * marketplace that uses `relevance` fields they don't recognize"; "a recognized field whose value
 * exceeds its limit ... invalidates the whole plugin entry". Limits count UTF-16 code units, the
 * length of a JavaScript string (spec/judgments.md). No signal matching happens here.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/relevance.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Relevance } from './types.ts';

const call = coreModule('format/marketplace/relevance');

/**
 * The phrase of "Working with <topic>?": `relevance.topic`, which "defaults to the plugin name with
 * each hyphen segment capitalized" (`terraform-helpers` gives `Terraform Helpers`).
 */
export const effectiveTopic = (entry: {
  readonly name: string;
  readonly relevance?: Relevance;
}): string => call('effectiveTopic', entry);
