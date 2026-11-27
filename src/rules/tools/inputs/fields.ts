/**
 * The tables of hooks.md ("PreToolUse input"), one per tool.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/fields.rs`,
 * docs/RUST.md). The zod schema of each field's value stays in the core, where `parseToolInput`
 * reads with it.
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/fields');

/** The JSON types of the tables of hooks.md ("PreToolUse input"). */
export type FieldType = 'string' | 'number' | 'boolean' | 'array' | 'object';

/**
 * What a rule can do with a field (permissions.md, "Match by input parameter"):
 * - `primary`: the content field of the tool; a parameter rule on it is ignored with a warning.
 * - `subject`: the field the tool's own specifier reads without being primary (the subagent type of
 *   `Agent(Explore)`); `Agent(subagent_type:Explore)` still matches it.
 * - `param`: any other scalar; a deny or ask `Tool(field:value)` rule matches it.
 * - `structured`: an array or object; "Fields nested inside an object or array are not matchable".
 */
export type FieldRole = 'subject' | 'param' | 'primary' | 'structured';

/** Where a field is documented: the SDK's input type, or only the docs (which the compiler cannot tie to the SDK). */
export type FieldSource = 'sdk' | 'docs';

export interface FieldSpec {
  readonly type: FieldType;
  readonly optional: boolean;
  readonly role: FieldRole;
  readonly source: FieldSource;
}

/** The tools hooks.md tabulates the input of. */
type InputTool =
  | 'Bash'
  | 'PowerShell'
  | 'Write'
  | 'Edit'
  | 'Read'
  | 'Glob'
  | 'Grep'
  | 'WebFetch'
  | 'WebSearch'
  | 'Agent'
  | 'AskUserQuestion'
  | 'ExitPlanMode';

type Fields = Readonly<Record<string, FieldSpec>>;

/**
 * One table per tool of hooks.md ("PreToolUse input"). Optionality is the table's ("Optional ...");
 * where it is silent (the boolean flags `run_in_background`, `replace_all`, `-i`, `multiline`, and
 * Agent, WebFetch and ExitPlanMode fields), the SDK's input type decides (spec/judgments.md).
 */
export const TOOL_INPUT_FIELDS: { readonly [T in InputTool]: Fields } = call('TOOL_INPUT_FIELDS');
