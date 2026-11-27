/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/grep.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/grep');

/** hooks.md, "Grep": "`content`, `files_with_matches`, or `count`". The one list of the modes. */
export const GREP_OUTPUT_MODES: readonly ['content', 'files_with_matches', 'count'] =
  call('GREP_OUTPUT_MODES');
export type GrepOutputMode = (typeof GREP_OUTPUT_MODES)[number];
