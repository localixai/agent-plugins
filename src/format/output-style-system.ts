/**
 * output-styles.md § How output styles work. The host supplies its private prompt parts; a style
 * replaces only coding-specific instructions unless it opts to keep them, and is appended intact.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/output_style_system.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PromptKind, StyleSelection } from './output-style-select.ts';

const call = coreModule('format/output-style-system');

/**
 * output-styles.md § How output styles work. The host supplies its private prompt parts; a style
 * replaces only coding-specific instructions unless it opts to keep them, and is appended intact.
 */
export const outputStyleSystem = (
  selection: StyleSelection,
  defaults: {
    readonly general: readonly string[];
    readonly coding: readonly string[];
    readonly promptKind?: PromptKind;
  },
): readonly string[] => call('outputStyleSystem', selection, defaults);
