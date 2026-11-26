import type { BuiltinModelInput } from './builtin-model.ts';
import type { ModelValueKind } from './model-value.ts';
import type { AgentDefinition, EffortLevel } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Which model a subagent runs on, and at which effort (sub-agents.md, "Choose a model", "Run
 * every subagent on one model" and "Choose an effort level"). Both are small decisions over
 * values the host passes: the kit reads no environment variable. Turning an alias into a version,
 * the `availableModels` allowlist and the effort levels a model supports are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/launch_model.rs`, docs/RUST.md).
 */
const call = coreModule('agents/launch-model');

/** Where the chosen model came from. */
export type ModelSource = 'invocation' | 'definition' | 'env' | 'builtin' | 'main';

export interface SubagentModelInput {
  /** The `model` parameter of this Agent tool call: an alias ("Claude can't pass a full model ID or `inherit`"). */
  readonly invocation?: string | undefined;
  /**
   * The model an installed mod set in its `agent.spawn` hook: "Claude Code uses that model in place of
   * the per-invocation parameter". Any model the mod names counts (it is no Claude call, so the alias
   * limit of the parameter does not bind it); judgment: ignored while the model is forced, because the
   * docs say only Claude's own call is withdrawn there.
   */
  readonly modModel?: string | undefined;
  /** The definition's `model`: an alias, a full model ID or `inherit`. */
  readonly definition?: string | undefined;
  /** `CLAUDE_CODE_SUBAGENT_MODEL`. */
  readonly envModel?: string | undefined;
  /** `CLAUDE_CODE_SUBAGENT_MODEL_FORCE` is `1`. */
  readonly force?: boolean | undefined;
  /** The main conversation's model, exactly (`[1m]` suffix included). */
  readonly mainModel: string;
  /** The main model's family (`opus`, `sonnet`...); undefined when Claude Code cannot tell it on another provider. */
  readonly mainFamily?: string | undefined;
  /** `ANTHROPIC_DEFAULT_OPUS_MODEL` is set, so `opus` means the model you set. */
  readonly opusPinned?: boolean | undefined;
  /**
   * Set when the subagent is a built-in (not a user or project agent of the same name, which keeps its
   * own `model`): its name and how the session connects. Explore on a Fable main conversation runs on
   * the `opus` alias over a subscription, Console or LLM gateway ("Built-in subagents").
   */
  readonly builtin?: BuiltinModelInput | undefined;
}

export interface SubagentModel {
  /** An alias or a full model ID; the main conversation's model (`mainModel`) when `kind` is `inherit`. */
  readonly model: string;
  readonly from: ModelSource;
  /**
   * What `model` is, so a host that routes models itself (another provider, its own model picker)
   * reads the choice as data: `inherit` runs the subagent on the main conversation's model, whatever
   * the host routes that to (`inherit` in the definition, the end of the order, or a family alias that
   * "resolves to the main conversation's model"); `alias` is a family alias the host resolves to a
   * version (`sonnet`, `opus`, `haiku`, `fable`); `full-id` is any other value, as written (it
   * "accepts the same values as the `--model` flag"). The `availableModels` check
   * (`checkAgainstAllowlist`) takes `model`.
   */
  readonly kind: ModelValueKind;
}

/** The model a subagent runs on, in "the subagent model order" of sub-agents.md. */
export const resolveSubagentModel = (input: SubagentModelInput): SubagentModel =>
  call('resolveSubagentModel', input);

export interface SubagentEffortInput {
  /** `CLAUDE_CODE_EFFORT_LEVEL`: a level, or `auto` / `unset`. */
  readonly env?: string | undefined;
  /** The `effort` parameter of this Agent tool call (a non-fork subagent only). */
  readonly invocation?: string | undefined;
  /** The definition's `effort`. */
  readonly field?: AgentDefinition['effort'];
  /** The level picked in the session (`/effort`, `--effort`, settings). */
  readonly session?: string | undefined;
  /** The subagent runs on the main conversation's model. */
  readonly sameModelAsMain: boolean;
}

/**
 * The effort a subagent runs at: the environment variable, then the invocation parameter, then
 * the `effort` field, then the session level only "on the main conversation's model" ("On another
 * model, that level doesn't apply"). `undefined` sends no level. The variable set to `auto` or
 * `unset` sends none, whatever the others say, because it "takes precedence over both".
 */
export const resolveSubagentEffort = (
  input: SubagentEffortInput,
): EffortLevel | number | undefined => call('resolveSubagentEffort', input);
