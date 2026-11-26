/**
 * What of a Claude Code plugin has no core counterpart (Agent Plugins Extended §18.4.5): commands,
 * agents, output styles, themes, LSP servers, monitors, workflows, `bin/`, `settings.json` and mods.
 * Each is reported with its path; the files stay where they are, for Claude Code (§18.4.1 item 4).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_core/components.rs`, docs/RUST.md).
 */
import type { Plugin } from '../../loading/plugin-model.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { notConverted } from '../notes.ts';
import type { ConversionNote } from '../types.ts';

const call = coreModule('convert/to-core/components');

/** How each note is made: its outcome and what it says after naming the component. */
export interface ComponentWording {
  readonly note: typeof notConverted;
  readonly says: string;
}

/**
 * One note per component; `mods`: the plugin's hooks file declares `modules`, which `Plugin` does not
 * carry.
 */
export const componentNotes = (
  plugin: Plugin,
  mods: boolean,
  wording?: ComponentWording,
): ConversionNote[] => call('componentNotes', plugin, mods, wording);
