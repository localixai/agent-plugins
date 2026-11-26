/**
 * The unquoted-placeholder warning of `claude plugin validate` on a shell-form hook command (oracle
 * 2.1.325, spec §3.2.3 item 5 and J3.20: the placeholders in order of first appearance; the changelog's "leaves `${CLAUDE_PLUGIN_ROOT}` unquoted (it breaks on plugin paths with
 * spaces)"). The validator reads three placeholders; a use inside single or double quotes is fine.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/shell_command.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/validate/shell-command');

/** The warning's text, or `undefined` when every placeholder is quoted. */
export const unquotedMessage = (command: string): string | undefined =>
  call('unquotedMessage', command);
