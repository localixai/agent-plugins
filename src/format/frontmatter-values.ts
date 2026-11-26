/**
 * Coercions of frontmatter values, shared by skills, commands, output styles and agents. YAML 1.2
 * (the core schema `parseFrontmatter` uses) reads `yes`, `no` and `on` as plain strings, so the
 * flag spellings Claude Code accepts are handled here, not by the parser.
 *
 * Each reader takes the field's key (for the problem it may report) and the raw value. An absent
 * value (`undefined`, or YAML `null`) is `undefined` with no problem; a value of the wrong shape is
 * `undefined` with a `FieldProblem`, so the caller decides whether to warn or stay silent.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/frontmatter_values.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

export interface FieldProblem {
  readonly key: string;
  /** Reads after the key: `${key} ${reason}`. */
  readonly reason: string;
}

export interface FieldRead<T> {
  readonly value: T | undefined;
  readonly problem?: FieldProblem;
}

const call = coreModule('format/frontmatter-values');

export const NOT_A_TABLE: string = call('NOT_A_TABLE');

/** `true`/`false`/`yes`/`no`/`on`/`off`/`1`/`0`, as a YAML boolean, number or string in any case. */
export const readFlag = (key: string, value: unknown): FieldRead<boolean> =>
  call('readFlag', key, value);

/** A string, trimmed; a number or boolean is written out; blank is absent. */
export const readText = (key: string, value: unknown): FieldRead<string> =>
  call('readText', key, value);

/**
 * How a string list is split. Which one a field uses is the docs' call: `allowed-tools` and
 * `disallowed-tools` (skills.md) are `space-or-comma`; `tools` and `disallowedTools`
 * (sub-agents.md) and `paths` (skills.md, memory.md) are `comma`, so `my dir/**` stays one item.
 */
export interface ListOptions {
  readonly separators: 'comma' | 'space-or-comma';
  /** Also keep `{...}` whole, for globs such as `src/**\/*.{ts,tsx}`. */
  readonly braces?: boolean;
}

/**
 * A list as Claude Code takes it: a string split as `options` say, outside parentheses, or a YAML
 * list whose items are kept whole. An item that is not text is dropped with a problem.
 */
export const readList = (
  key: string,
  value: unknown,
  options: ListOptions,
): FieldRead<readonly string[]> => call('readList', key, value, options);

export interface IntegerBounds {
  readonly min?: number;
  readonly max?: number;
}

/** A whole number, as a YAML number or a string of digits, within `min` and `max` when given. */
export const readInteger = (
  key: string,
  value: unknown,
  bounds: IntegerBounds = {},
): FieldRead<number> => call('readInteger', key, value, bounds);

/** A YAML table; anything else is dropped. */
export const readTable = (
  key: string,
  value: unknown,
): FieldRead<Readonly<Record<string, unknown>>> => call('readTable', key, value);

/** One of `allowed`, matched exactly after trimming (no case folding). */
export const readEnum = <T extends string>(
  key: string,
  value: unknown,
  allowed: readonly T[],
): FieldRead<T> => call('readEnum', key, value, allowed);
