import type { PermissionRule } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The startup warning for a Bash allow rule with a `*` before the subcommand (errors.md, "Has a
 * wildcard before the rest of the command").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/wildcard_warning.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/bash/wildcard-warning');

/**
 * The row shown below the startup banner in an interactive session (errors.md, "Has a wildcard
 * before the rest of the command"): `⚠ Allow rule Bash(git -C * status *) allows any option at its * · use an exact value`.
 */
export const wildcardBannerRow = (rule: PermissionRule): string => call('wildcardBannerRow', rule);
