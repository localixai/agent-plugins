/**
 * The `terminalSequence` field (hooks.md, "Emit terminal notifications"): "Restricted to OSC
 * `0`/`1`/`2`/`9`/`99`/`777` and BEL. If the value contains anything outside the allowlist, the
 * field is ignored." The value is "a string of one or more allowlisted escape sequences"; "Sequences
 * may be terminated with BEL or with ST", and CSI sequences, OSC palette sequences, OSC 8
 * hyperlinks, OSC 52 clipboard writes and OSC 1337 are rejected.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/terminal_sequence.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/terminal-sequence');

/** The value itself when every part is allowlisted, else `undefined`: the field is ignored. */
export const allowedTerminalSequence = (value: string): string | undefined =>
  call('allowedTerminalSequence', value);
