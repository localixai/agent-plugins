import type { ElicitationRequest, ElicitationResult } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The fields of a form-mode elicitation request and the check of an answer against them (mcp.md,
 * "Respond to MCP elicitation requests": "Claude Code shows a dialog with form fields defined by the
 * server"). Pure data: the dialog is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_elicitation_form.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/mcp-elicitation-form');

export type ElicitationFieldType =
  | 'string'
  | 'number'
  | 'integer'
  | 'boolean'
  | 'enum'
  | 'multi-enum';

export interface ElicitationField {
  readonly name: string;
  readonly type: ElicitationFieldType;
  readonly required: boolean;
  readonly title?: string;
  readonly description?: string;
  /** The allowed values of an `enum` or `multi-enum` field. */
  readonly options?: readonly string[];
  /** What a dialog shows for each of `options`, in order: the `title` of a `oneOf` entry, an `enumNames` entry, else the value. */
  readonly labels?: readonly string[];
  /** The schema's `default`, when it is a value a form answer can hold. */
  readonly default?: ElicitationValue;
  /** A string's `format`, one of `email`, `uri`, `date` and `date-time`. */
  readonly format?: ElicitationFormat;
  readonly minimum?: number;
  readonly maximum?: number;
  readonly minLength?: number;
  readonly maxLength?: number;
  /** A multi-select's bounds on how many options are chosen. */
  readonly minItems?: number;
  readonly maxItems?: number;
}

/** What MCP lets a form answer hold: text, a number, a boolean or a list of text. */
export type ElicitationValue = string | number | boolean | string[];

export const ELICITATION_FORMATS: readonly ['email', 'uri', 'date', 'date-time'] =
  call('ELICITATION_FORMATS');
export type ElicitationFormat = (typeof ELICITATION_FORMATS)[number];

/** The fields of a form request: its `requestedSchema` properties, in order, with `required` marked. */
export const elicitationFormFields = (
  schema: Readonly<Record<string, unknown>> | undefined,
): readonly ElicitationField[] => call('elicitationFormFields', schema);

export const contentOf = (
  values: Readonly<Record<string, unknown>>,
): Record<string, ElicitationValue> => call('contentOf', values);

/**
 * The problem each field's value has, by field name; empty when the values fit. A field with no value
 * is a problem only when it is required. Judgment (spec/judgments.md): the wording is the kit's.
 */
export const elicitationFieldProblems = (
  schema: Readonly<Record<string, unknown>> | undefined,
  values: Readonly<Record<string, unknown>>,
): Readonly<Record<string, string>> => call('elicitationFieldProblems', schema, values);

export type ElicitationAnswer =
  | { readonly result: ElicitationResult }
  | { readonly problems: readonly string[] };

export const formAnswer = (
  request: ElicitationRequest,
  values: Readonly<Record<string, unknown>>,
): ElicitationAnswer => call('formAnswer', request, values);
