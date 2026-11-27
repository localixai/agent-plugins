import type { StoreLayout } from './store-layout.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The exact texts of the install and loading problems the docs quote (plugins/troubleshooting.md,
 * plugins/loading.md, plugins/cli-reference.md): pure builders, each named for the heading it
 * answers. A text the docs quote is asserted verbatim in tests/marketplace/install-messages.test.ts;
 * the join between a quoted message and a quoted hint or fix, which the docs do not show, is a row
 * in spec/judgments.md. Messages the Errors tab of `/plugin` shows come as `ErrorsTabEntry`: the
 * message and the guidance line beneath it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_messages.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/install-messages');

/** An entry of the `/plugin` **Errors** tab: "Each entry pairs a message with a guidance line." */
export interface ErrorsTabEntry {
  readonly message: string;
  readonly guidance: string;
}

// ---- installed_plugins.json ----

/**
 * "installed_plugins.json holds a record under "<id>" that this version of Claude Code cannot
 * read"; with several, "holds records under" (troubleshooting.md). The options after it are the
 * three steps of that section, in order.
 */
export const unreadableRecordsMessage = (ids: readonly string[]): string =>
  call('unreadableRecordsMessage', ids);

/** "The whole file declares a format this version doesn't know" (troubleshooting.md). */
export const unknownFormatMessage = (version: number): string =>
  call('unknownFormatMessage', version);

// ---- the commands that refuse, and the notes `claude plugin list` prints ----

/** The three commands that refuse while a record cannot be read, with the word their refusal uses. */
export type RefusingCommand = 'install' | 'uninstall' | 'update';

/** `failureCode` of `--json` on those three commands when a record cannot be read. */
export const RECORDS_UNREADABLE_CODE: 'install_records_unreadable' =
  call('RECORDS_UNREADABLE_CODE');

export interface RecordsRefusal {
  /** `Plugin "<name>" was not installed:` (or `uninstalled`, `updated`) and the same text `claude plugin list` prints. */
  readonly message: string;
  /** The result line of `--json` carries this message and this `failureCode`. */
  readonly failureCode: typeof RECORDS_UNREADABLE_CODE;
}

/**
 * How `claude plugin install`, `uninstall` and `update` refuse (troubleshooting.md): `Plugin
 * "<name>" was not installed:`, `was not uninstalled:` or `was not updated:`, followed by the
 * unreadable-record text; with `--json` the result carries that `message` and `failureCode`.
 */
export const refuseForUnreadableRecords = (
  command: RefusingCommand,
  name: string,
  reason: string,
): RecordsRefusal => call('refuseForUnreadableRecords', command, name, reason);

/**
 * The last line of stdout of `claude plugin install|uninstall|update --json` for that refusal: "one
 * JSON object" with `command`, `outcome` and `message`, and `failureCode` where it applies
 * (cli-reference.md, "JSON result format"; troubleshooting.md: "the result line carries the same
 * `message` and `failureCode`").
 */
export const refusalResultLine = (command: RefusingCommand, refusal: RecordsRefusal): string =>
  call('refusalResultLine', command, {
    message: refusal.message,
    failureCode: refusal.failureCode,
  });

/** `claude plugin list` prints the unreadable-record text as a `Note:`. */
export const listNote = (text: string): string => call('listNote', text);

/**
 * The note naming a dated copy that sits beside `installed_plugins.json`. Judgment
 * (spec/judgments.md): the words around the quoted heading and the path.
 */
export const rebuiltNote = (keptPath: string): string => call('rebuiltNote', keptPath);

/** "The note ends `Nothing needs doing about these copies.`" (troubleshooting.md). */
export const setAsideNote = (copyPath: string): string => call('setAsideNote', copyPath);

/**
 * The notes `claude plugin list` prints "for as long as" a dated copy "sits beside
 * `installed_plugins.json`" (troubleshooting.md): one per `.unreadable` copy and one per
 * `.set-aside` copy found among `names`, the files directly under the plugins root. `layout`
 * crosses as the host's object: its `file` names each copy's path.
 */
export const recoveryNotes = (names: readonly string[], layout: StoreLayout): readonly string[] =>
  call('recoveryNotes', names, layout);
