/**
 * The coercers of an agent's scalar frontmatter fields (sub-agents.md, "Supported frontmatter
 * fields"), each over a shared reader of `frontmatter-values.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_scalars.rs`,
 * docs/RUST.md).
 */
import type { EffortLevel } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/agent-scalars');

/** What one agent field read gives: its value, or `undefined` with the issues that say why. */
export interface AgentFieldRead<T> {
  readonly value: T | undefined;
  readonly issues: readonly PluginIssue[];
}

/** The coercer of a field: `path` is the agent file, for the issues it reports. */
export type AgentCoercer<T> = (path: string, key: string, raw: unknown) => AgentFieldRead<T>;

export const textField: AgentCoercer<string> = (path, key, raw) =>
  call('textField', path, key, raw);

/**
 * `model`: any non-empty text (spec §5.4.3, J5.5). An empty or blank value is dropped like any bad
 * value, with the warning the client logs, and the default applies.
 */
export const modelField: AgentCoercer<string> = (path, key, raw) =>
  call('modelField', path, key, raw);

/**
 * `background`, `omitClaudeMd`: sub-agents.md says "Set to `true`", so only a YAML boolean or the
 * text `true` or `false` is a flag here; `yes`, `on`, `1` and `0` belong to skills and commands
 * (judgment, spec/judgments.md).
 */
export const boolField: AgentCoercer<boolean> = (path, key, raw) =>
  call('boolField', path, key, raw);

/** `tools`, `disallowedTools`, `skills`: "a comma-separated string ... or a YAML list". */
export const tokenList: AgentCoercer<string[]> = (path, key, raw) =>
  call('tokenList', path, key, raw);

/** A whole number of at least 1. */
export const positiveInt: AgentCoercer<number> = (path, key, raw) =>
  call('positiveInt', path, key, raw);

/** One of `allowed`, exactly. */
export function enumField<T extends string>(allowed: readonly T[]): AgentCoercer<T> {
  return (path, key, raw) => call('enumField', allowed, path, key, raw);
}

/** sub-agents.md, "Choose an effort level": `low`, `medium`, `high`, `xhigh` or `max`. */
export const effortField: AgentCoercer<EffortLevel> = (path, key, raw) =>
  call('effortField', path, key, raw);

/** sub-agents.md, "Set the auto-compact window for one subagent": a token count in this range. */
export const AUTO_COMPACT_RANGE: { readonly min: 100_000; readonly max: 1_000_000 } =
  call('AUTO_COMPACT_RANGE');

/**
 * `autoCompactWindow`: from 100000 to 1000000. A value out of that range (or not a number) leaves
 * the subagent loading "as if the field were absent", reported with the words the docs tell the
 * author to search the debug log for: `invalid autoCompactWindow`.
 */
export const autoCompactWindow: AgentCoercer<number> = (path, key, raw) =>
  call('autoCompactWindow', path, key, raw);

/** `experimental.cacheTtl`: "`5m` or `1h`". */
export type CacheTtl = '5m' | '1h';

/** What the kit keeps of the `experimental` map: only `cacheTtl`. */
export interface Experimental {
  readonly cacheTtl?: CacheTtl;
}

/**
 * The `experimental` map: "Claude Code ignores any other value", so another key is dropped
 * silently, and a `cacheTtl` that is not `5m` or `1h` is dropped with a warning (judgment).
 */
export const experimentalMap: AgentCoercer<Experimental> = (path, key, raw) =>
  call('experimentalMap', path, key, raw);
