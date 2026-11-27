/**
 * The launcher of `processWrapper` and `CLAUDE_CODE_PROCESS_WRAPPER` (settings-reference.md,
 * `processWrapper`; corporate-launcher.md, "Format of the launcher value"). Both take the same format:
 * "Claude Code parses the value as an argument list, not a shell command".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/process_wrapper.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/general/process-wrapper');

export type LauncherRead =
  | { readonly argv: readonly string[] }
  /** The value cannot be used: Claude Code "refuses to start the affected process" and reports this. */
  | { readonly problem: string };

/** The per-session launcher, ignored on Windows. */
export const readProcessWrapper = (
  view: PolicyView,
  options: {
    readonly env: Readonly<Record<string, string | undefined>>;
    readonly windows: boolean;
  },
): LauncherRead | undefined =>
  call('readProcessWrapper', view, { env: options.env, windows: options.windows });
