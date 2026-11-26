/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/exit_plan_mode.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/exit-plan-mode');

/**
 * What ExitPlanMode runs with (hooks.md, "ExitPlanMode"): `allowedPrompts` is "Deprecated. Claude
 * Code accepts the field but ignores it", so the effective input is the input without it. Anything
 * that is not an object has no fields (a judgment, spec/judgments.md).
 */
export const effectiveExitPlanModeInput = (input: unknown): Record<string, unknown> =>
  call('effectiveExitPlanModeInput', input);
