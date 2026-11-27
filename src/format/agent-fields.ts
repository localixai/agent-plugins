import type { HooksSettings } from '../protocol/hook-config.ts';
import type { AgentDefinition, AgentMcpServerSpec, EffortLevel } from '../protocol/official.ts';
import type { Assert, Equal } from '../protocol/type-equality.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { AgentCoercer, Experimental } from './agent-scalars.ts';

/**
 * The 19 frontmatter fields of a subagent file (sub-agents.md, "Frontmatter reference"), in the
 * table's order. Claude Code "ignores a field it doesn't recognize", so anything else is
 * `unrecognized`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/agent_fields.rs`, docs/RUST.md).
 */

const call = coreModule('format/agent-fields');

/** The fields, in the table's order. */
export const AGENT_FIELD_NAMES: readonly [
  'name',
  'description',
  'tools',
  'disallowedTools',
  'model',
  'permissionMode',
  'maxTurns',
  'autoCompactWindow',
  'skills',
  'mcpServers',
  'hooks',
  'memory',
  'background',
  'omitClaudeMd',
  'effort',
  'isolation',
  'color',
  'initialPrompt',
  'experimental',
] = call('AGENT_FIELD_NAMES');
export type AgentFieldName = (typeof AGENT_FIELD_NAMES)[number];

/** `color`: "Accepts `red`, `blue`, `green`, `yellow`, `purple`, `orange`, `pink`, or `cyan`". */
export const AGENT_COLORS: readonly [
  'red',
  'blue',
  'green',
  'yellow',
  'purple',
  'orange',
  'pink',
  'cyan',
] = call('AGENT_COLORS');
export type AgentColor = (typeof AGENT_COLORS)[number];

/** `isolation`: "Set to `worktree`"; plugins/components.md: "The only valid `isolation` value". */
export const AGENT_ISOLATIONS: readonly ['worktree'] = call('AGENT_ISOLATIONS');
export type AgentIsolation = (typeof AGENT_ISOLATIONS)[number];

/** `memory`: "`user`, `project`, or `local`", tied to the official `AgentDefinition`. */
export const AGENT_MEMORY_SCOPES: readonly ['user', 'project', 'local'] =
  call('AGENT_MEMORY_SCOPES');
export type AgentMemoryScope = (typeof AGENT_MEMORY_SCOPES)[number];

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type MemoryScopesMatchSdk = Assert<
  Equal<AgentMemoryScope, NonNullable<AgentDefinition['memory']>>
>;

/** @public The `effort` field names the same levels as the SDK (a number is the SDK's extra). */
export type AgentEffortMatchesSdk = Assert<
  Equal<Extract<NonNullable<AgentDefinition['effort']>, string>, EffortLevel>
>;

/** What each field reads to. */
interface AgentFieldValues {
  readonly name: string;
  readonly description: string;
  readonly tools: string[];
  readonly disallowedTools: string[];
  readonly model: string;
  readonly permissionMode: NonNullable<AgentDefinition['permissionMode']>;
  readonly maxTurns: number;
  readonly autoCompactWindow: number;
  readonly skills: string[];
  readonly mcpServers: AgentMcpServerSpec[];
  readonly hooks: HooksSettings;
  readonly memory: AgentMemoryScope;
  readonly background: boolean;
  readonly omitClaudeMd: boolean;
  readonly effort: EffortLevel;
  readonly isolation: AgentIsolation;
  readonly color: AgentColor;
  readonly initialPrompt: string;
  readonly experimental: Experimental;
}

export type AgentFieldValue<F extends AgentFieldName> = AgentFieldValues[F];

export interface AgentFieldSpec<F extends AgentFieldName> {
  /**
   * plugins/components.md, "Frontmatter fields in plugin agents": a plugin agent applies a
   * `supported` field and ignores an `ignored` one.
   */
  readonly plugin: 'supported' | 'ignored';
  /** Coerces the raw YAML value; `path` is the agent file, for the issues. */
  readonly read: AgentCoercer<AgentFieldValue<F>>;
}

/** The registry: one spec per field name, so a field added to the table fails the typecheck here. */
export const AGENT_FIELDS: { readonly [F in AgentFieldName]: AgentFieldSpec<F> } =
  call('AGENT_FIELDS');

/** The fields a plugin agent ignores (plugins/components.md): derived from the registry. */
export const PLUGIN_IGNORED_FIELDS: readonly AgentFieldName[] = call('PLUGIN_IGNORED_FIELDS');
