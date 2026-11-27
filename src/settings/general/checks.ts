import type { RepairAt, Repaired } from '../repair/support.ts';
import type { GeneralKey } from './keys.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * What a settings file may hold under each general key, checked as Claude Code does ("Settings
 * Warning: only individual entries fail, such as a malformed permission rule or an unknown hook
 * event name. Claude Code skips those values and keeps the rest of the file in effect", settings.md,
 * "Fix a broken settings file"). A value of the wrong type is dropped with a warning that names
 * the key; a malformed entry of a list is skipped on its own. In the policy tier a key that fails
 * closed reads as its restrictive value instead (managed-settings.md, "Keys that fail closed"). The
 * types are the SDK's `Settings` (docs/ARCHITECTURE.md, E2): these guards only read them at run time.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/checks.rs`,
 * docs/RUST.md).
 */
const call = coreModule('settings/general/checks');

/**
 * The check of a general key run on `value`. A dotted key lives inside a block and is read by its
 * row's own guard (`rows.ts`), so a file's top-level name of that shape is kept as written.
 */
export const generalCheck = (key: GeneralKey, value: unknown, at: RepairAt): Repaired =>
  call('generalCheck', key, value, at);
