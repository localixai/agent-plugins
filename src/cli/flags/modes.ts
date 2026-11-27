import type { Customization } from './types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The modes that leave customizations out (cli-reference.md, `--bare`, `--safe-mode`, `--restricted`,
 * `--disable-slash-commands`): which kinds each skips (`customizations.ts`), the variable it sets, and
 * for `--restricted` the rest of its list. What loads under the settings, tool and permission rules
 * is those modules' (`availableTools` reads `restricted`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/modes.rs`, docs/RUST.md).
 */

const call = coreModule('cli/flags/modes');

/** Whether a safe-mode session still applies the managed policy of a kind it skips; `undefined` where the docs say nothing. */
export const managedPolicyUnderSafeMode = (kind: Customization): boolean | undefined =>
  call('managedPolicyUnderSafeMode', kind);
