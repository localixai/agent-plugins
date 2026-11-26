import { coreModule } from '../wasm/bridge.ts';
import type { ElicitationField } from './mcp-elicitation-form.ts';

/**
 * The check of one form value against its field (mcp.md, "Respond to MCP elicitation requests": the
 * person "fills in the fields and submits"). The docs do not say how the client checks them; the
 * constraints are those of the MCP form schema (`minimum`, `maximum`, `minLength`, `maxLength`,
 * `format`, `minItems`, `maxItems`, the listed options), and the wording is the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_elicitation_check.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/mcp-elicitation-check');

/** The problem `value` has as the value of `field`, or `undefined` when it fits. */
export const fieldProblem = (field: ElicitationField, value: unknown): string | undefined =>
  call('fieldProblem', field, value);
