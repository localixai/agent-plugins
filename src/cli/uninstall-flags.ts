/**
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/uninstall_flags.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/uninstall-flags');

/** The flags of `claude plugin uninstall` that bear on whether it can run. */
interface UninstallFlags {
  readonly prune?: boolean;
  readonly yes?: boolean;
  readonly json?: boolean;
}

/** `--json` and a missing `-y` are flag problems: nothing is printed but the reason. */
export const flagProblem = (args: UninstallFlags, tty: boolean): string | undefined =>
  call('flagProblem', args, tty);
