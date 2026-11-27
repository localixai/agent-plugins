import { coreModule } from '../../wasm/bridge.ts';

/**
 * `PowerShell(...)` rules with a command pattern: no wrappers, aliases resolved on both sides, case
 * ignored; `&` and `.` run the command named after them, and a module-qualified name is the cmdlet.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/powershell.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/powershell');

/** The command (or pattern) with its first word, when it names a cmdlet another way, as the cmdlet. */
export const canonicalizePowerShell = (command: string): string =>
  call('canonicalizePowerShell', command);
