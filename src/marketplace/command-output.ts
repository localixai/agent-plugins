import { coreModule } from '../wasm/bridge.ts';

/**
 * What a `command` source printed (plugins/marketplace-reference.md, "What the command must do",
 * "Output that fails the install or update"): "exactly one line on stdout, the absolute path of the
 * plugin directory", not a UNC path on Windows, and not the directory Claude Code was started in
 * or one of its parents. Pure text handling; the messages are the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/command_output.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/command-output');

type Platform = 'posix' | 'windows';

/** True when two spellings name one folder on `platform`: on Windows separators and capitals do not tell folders apart. */
export const samePath = (a: string, b: string, platform: Platform): boolean =>
  call('samePath', a, b, platform);
