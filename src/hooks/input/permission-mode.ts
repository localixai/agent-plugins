import type { PermissionMode } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/input/permission-mode');

/**
 * The `permission_mode` a hook receives (hooks.md, "Common input fields"): one of `"default"`,
 * `"plan"`, `"acceptEdits"`, `"auto"`, `"dontAsk"` or `"bypassPermissions"`. "The mode labeled
 * **Manual** arrives as `"default"`, never as `"manual"`, so scripts that match `"default"` keep
 * working." A host that labels its mode Manual passes it as it is called; a text that names no
 * mode yields `undefined`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/input/permission_mode.rs`,
 * docs/RUST.md).
 */
export const hookPermissionMode = (mode: string): PermissionMode | undefined =>
  call('hookPermissionMode', mode);
