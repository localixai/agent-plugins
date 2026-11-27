/**
 * A managed `sandbox` block, validated field by field (managed-settings.md, "Invalid values inside
 * `sandbox`"): one invalid value never drops the block. A quoted Boolean counts as that Boolean; an
 * invalid Boolean reads as the value that keeps the sandbox strictest; an invalid list entry is
 * dropped and a list with no valid entry does not apply; an invalid deny list withholds the allow
 * lists it restrains.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/sandbox_check.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { Repair } from '../repair/support.ts';

const call = coreModule('settings/general/sandbox-check');

/**
 * `sandbox`: outside the policy tier the block stays as written (a managed-only lock inside it is
 * left out by its own row's scope line, `readSettingsLayer`); in the policy tier it is repaired
 * field by field.
 */
export const sandbox: Repair = (value, at) => call('sandbox', value, at);
