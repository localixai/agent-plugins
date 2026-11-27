import type { PluginIssue } from './issues.ts';
import type { JsToken } from './js-tokens.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The `meta` block of a workflow script (workflows.md, "What the saved script looks like", "Edit a
 * saved script"; agent-sdk/typescript.md, "Workflow": the script "must begin with `export const meta
 * = { name, description }` as a literal"). Claude Code reads it without running the script, so the
 * kit does too: it finds the literal in the tokens, reads it, and rewrites single fields in place,
 * keeping every other byte.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/workflow_meta.rs`, docs/RUST.md).
 */

const call = coreModule('format/workflow-meta');

/** A `phases` entry as written: a title, or an object with a `title` and whatever else it carries (judgment). */

/** A `phases` entry as written: a title, or an object with a `title` and whatever else it carries (judgment). */
export type PhaseEntry = string | { readonly title: string; readonly [field: string]: unknown };

export interface WorkflowMeta {
  readonly name: string;
  readonly description: string;
  /** The stage titles of `phases`, in order; `[]` when there are none. */
  readonly phases: readonly string[];
  /** The entries behind `phases`, as written, so an object entry keeps its other fields (judgment). */
  readonly phaseEntries: readonly PhaseEntry[];
  /** `whenToUse`, when it is a string (judgment: the docs do not name the field). */
  readonly whenToUse: string | undefined;
  /** Every other literal field, as written. */
  readonly extra: Readonly<Record<string, unknown>>;
}

/** One property of the `meta` literal, and where its value is in the source. */
interface LiteralProperty {
  readonly key: string;
  readonly value: unknown;
  /** Where the value starts and ends in the source. */
  readonly valueStart: number;
  readonly valueEnd: number;
}

/** The `meta` literal and where its parts are in the source. */
interface LiteralObject {
  /** The `{` and the end of the `}`. */
  readonly start: number;
  readonly end: number;
  /** The position of the `}` itself. */
  readonly closeAt: number;
  readonly properties: readonly LiteralProperty[];
  /** The last property is followed by a comma. */
  readonly trailingComma: boolean;
  /** The end of the last property and of its comma, if any; just after the `{` when there is none. */
  readonly afterLast: number;
  /** The index of the token after the `}`. */
  readonly endIndex: number;
}

/** A script split at the end of its `meta` statement. */
export interface ScriptHead {
  readonly meta: WorkflowMeta | undefined;
  readonly issues: readonly PluginIssue[];
  /** The tokens of the whole script, or of its head up to the `meta` statement's end (`ScriptScope`). */
  readonly tokens: readonly JsToken[];
  /** The literal, when the script begins with one. */
  readonly object: LiteralObject | undefined;
  /** Where the `meta` statement ends (after its `;`); the body starts here. `0` without a literal. */
  readonly headEnd: number;
  /** The index of the first token of the body. */
  readonly bodyIndex: number;
}

/** How much of a script to tokenize: the `meta` statement only, or all of it for a scan of the body. */
export type ScriptScope = 'head' | 'whole';

/**
 * Splits a script at the end of its `meta` statement and reads the literal. What the docs leave open
 * (the severity of each finding, blank names) is in spec/judgments.md. A lexical problem (an
 * unterminated string, template or comment) is the runtime's to report when it lies after the `meta`
 * statement ("If the body has a syntax error, Claude Code reports it when you run the workflow"): the
 * `meta` is still read. When the literal cannot be read at all, the problem may be what broke it, so
 * it is the error (judgment).
 */
export const readScriptHead = (source: string, scope: ScriptScope): ScriptHead =>
  call('readScriptHead', source, scope);

/** The `meta` of a script and what is wrong with it. Never runs the script, never throws. */
export const readWorkflowMeta = (
  source: string,
): {
  readonly meta: WorkflowMeta | undefined;
  readonly issues: readonly PluginIssue[];
} => call('readWorkflowMeta', source);
