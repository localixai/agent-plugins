/**
 * The format of a tool's rule specifier, and what each format changes about reading one before any
 * matching (permissions.md, "Tool-specific permission rules").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/specifier_formats.rs`, docs/RUST.md).
 */
import type { AgentInput, BashInput, MonitorInput } from '../protocol/official-tools.ts';
import type { Assert, Equal } from '../protocol/type-equality.ts';
import { coreModule } from '../wasm/bridge.ts';

import { RULE_FORMATS, type RuleFormatId } from './tools/rule-formats.ts';

export { RULE_FORMATS, type RuleFormatId };

const call = coreModule('rules/specifier-formats');

/** The format of a tool's rule specifier: its catalog entry (`tools/catalog.ts`); any other tool takes only its bare name. */
export const ruleFormatOf = (toolName: string): RuleFormatId => call('ruleFormatOf', toolName);

/**
 * The input fields of the command tools: a `name:value` specifier of a command rule is a parameter
 * rule only for one of these, because `git:* push` or `npm:run` is command text and the docs do
 * not say otherwise (spec/judgments.md). Bash and Monitor share a rule format; their fields are
 * the SDK's `BashInput` and `MonitorInput`, and the guards below fail the typecheck when either
 * drifts. The SDK has no PowerShell input type, so PowerShell reads Bash's fields (judgment).
 */
type BashInputField =
  | 'command'
  | 'timeout'
  | 'description'
  | 'run_in_background'
  | 'dangerouslyDisableSandbox';

type MonitorInputField = 'description' | 'timeout_ms' | 'command' | 'ws';

/** @public A compile-time drift guard: a field the SDK adds to or drops from `BashInput` fails the typecheck. */
export type BashFieldsMatchSdk = Assert<Equal<BashInputField, keyof BashInput>>;

/** @public A compile-time drift guard: a field the SDK adds to or drops from `MonitorInput` fails the typecheck. */
export type MonitorFieldsMatchSdk = Assert<Equal<MonitorInputField, keyof MonitorInput>>;

/**
 * The input fields of `Agent`: a `name:value` specifier of an Agent rule is a parameter rule only
 * for one of these, so a plugin-qualified subagent (`Agent(acme:reviewer)`) stays a name. The SDK's
 * `AgentInput` is the list, and the guard fails the typecheck when it drifts.
 */
type AgentInputField =
  | 'description'
  | 'prompt'
  | 'subagent_type'
  | 'model'
  | 'effort'
  | 'run_in_background'
  | 'name'
  | 'team_name'
  | 'mode'
  | 'isolation';

/** @public A compile-time drift guard: a field the SDK adds to or drops from `AgentInput` fails the typecheck. */
export type AgentFieldsMatchSdk = Assert<Equal<AgentInputField, keyof AgentInput>>;
