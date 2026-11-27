import { type Decision } from '../../rules/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { HookOutcome } from '../outcome.ts';

/**
 * Implemented by the Rust core
 * (`crates/agent-plugins/src/hooks/output/permission_request_outcome.rs`, docs/RUST.md).
 */
const call = coreModule('hooks/output/permission-request-outcome');

/**
 * The decision once the `PermissionRequest` hooks have answered (hooks.md, "PermissionRequest
 * decision control"). The hooks answer the prompt a call would get, and also where no prompt may be
 * shown: "Anything that would prompt is denied unless a `PermissionRequest` hook allows it"
 * (headless.md), and with `permissions.denyInsteadOfPrompting` "A `PermissionRequest` hook still
 * answers" (settings-reference.md). A call denied before any hook is asked (a deny rule, a
 * critical-path removal in auto mode) and a call already decided stay as they are; with no answer
 * the decision stands. The `updatedInput` and `updatedPermissions` of an allow stay in `effects`.
 */
export const permissionRequestDecision = (
  decision: Decision,
  outcome: Pick<HookOutcome<'PermissionRequest'>, 'effects'>,
): Decision => call('permissionRequestDecision', decision, outcome);
