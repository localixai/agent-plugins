import {
  refuseForUnreadableRecords,
  type RefusingCommand,
} from '../marketplace/install-messages.ts';
import { commandFailed, type CommandOutput } from './result.ts';

/**
 * While `installed_plugins.json` holds a record this version cannot read, or is in a format it does
 * not know, `claude plugin install`, `uninstall` and `update` refuse before anything else (§16.6.2
 * item 1): `Plugin "<name>" was not installed:` (`uninstalled`, `updated`) and the text `plugin
 * list` prints as its note, with `failureCode: "install_records_unreadable"`. `unreadable` is that
 * text, as `readInstalledPlugins` reported it; `undefined` when every record was read.
 */
export function recordsRefusal(
  command: RefusingCommand,
  name: string,
  unreadable: string | undefined,
): CommandOutput | undefined {
  if (unreadable === undefined) return undefined;
  const refusal = refuseForUnreadableRecords(command, name, unreadable);
  return commandFailed(command, refusal.message, { failureCode: refusal.failureCode });
}
