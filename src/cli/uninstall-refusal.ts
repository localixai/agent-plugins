/**
 * The text after `"<name>" was not uninstalled: ` when a settings file of the scope still switches,
 * or may still switch, the plugin on (errors.md, "Plugin was not uninstalled").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/uninstall_refusal.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/uninstall-refusal');

/**
 * Why a settings file of the scope may still switch the plugin on (errors.md, "Plugin was not
 * uninstalled"): `still-on` the write reported success but the entry is still there; `change-failed`
 * the file could not be saved, for `error`; `unreadable` it exists but is not readable as settings;
 * `not-read` it is on a network path or a link to one, or its path could not be examined.
 */
export type StillSwitchedOn =
  | { readonly file: string; readonly cause: 'still-on' | 'unreadable' | 'not-read' }
  | { readonly file: string; readonly cause: 'change-failed'; readonly error: string };

/** The cause and the sentence that says what to do, for one settings file. */
export const stillOnReason = (fact: StillSwitchedOn): string => call('stillOnReason', fact);
