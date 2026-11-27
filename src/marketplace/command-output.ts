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

export type PrintedPath = { readonly path: string } | { readonly problem: string };

/** The absolute path the command printed, or what is wrong with its output. */
export const readPrintedPath = (stdout: string, platform: Platform): PrintedPath =>
  call('readPrintedPath', stdout, platform);

/** True when two spellings name one folder on `platform`: on Windows separators and capitals do not tell folders apart. */
export const samePath = (a: string, b: string, platform: Platform): boolean =>
  call('samePath', a, b, platform);

/** True when `printed` is `sessionDirectory` or one of its parents ("the session's own directory"). */
export const containsSession = (
  printed: string,
  sessionDirectory: string | undefined,
  platform: Platform,
): boolean => call('containsSession', printed, sessionDirectory, platform);
