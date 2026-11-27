/**
 * `hooks` of a settings file, read with the hooks reader so a settings file and a plugin's
 * `hooks.json` agree on what loads.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/hooks.rs`, docs/RUST.md).
 */
import type { Repair } from './support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/repair/hooks');

/**
 * `hooks`, read with the hooks reader so a settings file and a plugin's `hooks.json` agree on what
 * loads. The policy tier (managed, parent) ignores the whole field on any error: errors.md says
 * "when a line lists the source's `hooks` field as ignored, none of that source's hooks run". Any
 * other source skips only the entries that fail, handler by handler, and keeps the rest of the file
 * in effect (settings.md, "Fix a broken settings file": "only individual entries fail, such as ...
 * an unknown hook event name. Claude Code skips those values and keeps the rest of the file in
 * effect"). A `hooks` value that is not an object leaves nothing to keep. Each issue names the scope.
 */
export const hooks: Repair = (value, at) => call('hooks', value, at);
