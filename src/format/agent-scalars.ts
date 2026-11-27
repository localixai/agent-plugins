/**
 * The coercers of an agent's scalar frontmatter fields (sub-agents.md, "Supported frontmatter
 * fields"), each over a shared reader of `frontmatter-values.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_scalars.rs`,
 * docs/RUST.md).
 */
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

/** sub-agents.md, "Set the auto-compact window for one subagent": a token count in this range. */
export const AUTO_COMPACT_RANGE: { readonly min: 100_000; readonly max: 1_000_000 } =
  call('AUTO_COMPACT_RANGE');

/** `experimental.cacheTtl`: "`5m` or `1h`". */
export type CacheTtl = '5m' | '1h';

/** What the kit keeps of the `experimental` map: only `cacheTtl`. */
export interface Experimental {
  readonly cacheTtl?: CacheTtl;
}
