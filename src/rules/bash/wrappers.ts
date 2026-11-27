import type { PermissionBehavior } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The wrappers permissions.md ("Bash > Wrappers") says Claude Code strips before matching:
 * `timeout`, `time`, `nice`, `nohup` and `stdbuf`, the shell builtins `command` and `builtin`, zsh's
 * `noglob`, and bare `xargs`. "This wrapper list is built in and is not configurable."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/wrappers.rs`, docs/RUST.md).
 */
const call = coreModule('rules/bash/wrappers');

/** The command a subcommand runs once every leading wrapper and assignment is removed, as written. */
export const stripWrappers = (
  subcommand: string,
  behavior: PermissionBehavior,
  safeEnv?: ReadonlySet<string>,
): string => call('stripWrappers', subcommand, behavior, safeEnv);
