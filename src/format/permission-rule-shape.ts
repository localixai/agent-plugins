/**
 * The shape of a permission rule's text, shared by the settings reader (a malformed rule is a
 * Settings Warning) and the rules parser (permissions.md, "Permission rule syntax"; errors.md,
 * "Malformed Tool(content) rule").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/permission_rule_shape.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/permission-rule-shape');

export interface RuleParts {
  readonly toolName: string;
  readonly content: string | undefined;
}

/** errors.md ("Malformed Tool(content) rule"), quoted. */
export const MALFORMED_RULE: string = call('MALFORMED_RULE');

/** errors.md: `Invalid permission rule "..." was skipped: <reason>`. */
export const invalidRuleMessage = (raw: string, reason: string): string =>
  call('invalidRuleMessage', raw, reason);

/**
 * `Tool` or `Tool(content)`. Parentheses inside the content are literal, so the rule ends at its
 * last character, which must be the closing parenthesis (permissions.md, "Permission rule syntax");
 * `undefined` for text of no such shape.
 */
export const splitRule = (raw: string): RuleParts | undefined => call('splitRule', raw);
