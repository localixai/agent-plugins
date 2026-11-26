/**
 * The forms of a `model` value (sub-agents.md, "Choose a model"): "Model alias: use one of the
 * available aliases: `sonnet`, `opus`, `haiku`, or `fable`", "Full model ID ... Accepts the same
 * values as the `--model` flag", and "inherit: use the same model as the main conversation". Shared
 * by the model order (`launch-model.ts`) and the writers and allowlist (`model-choice.ts`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/model_value.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/model-value');

/** The family aliases an Agent tool call's `model` parameter takes. */
export const MODEL_ALIASES: readonly ['sonnet', 'opus', 'haiku', 'fable'] = call('MODEL_ALIASES');
export type ModelAlias = (typeof MODEL_ALIASES)[number];

/**
 * The family alias of a model: the alias itself, or the one a model ID embeds (`claude-opus-5-5`,
 * `claude-opus-5-5[1m]` and `my-gateway/claude-opus-5-5` are `opus`), read as `availableModels`
 * reads a family entry (model-config.md, "Add a custom model option": "A custom ID that embeds a
 * family name ... counts as a specific entry for that family"). `undefined` when it embeds none, as
 * an inference profile ARN Claude Code "hasn't resolved to a backing model" (sub-agents.md, "Choose
 * a model"): the `mainFamily` of `resolveSubagentModel` is then left out.
 */
export const modelFamily = (model: string): ModelAlias | undefined => call('modelFamily', model);

/** The three forms the frontmatter `model` field takes. */
export type ModelValueKind = 'alias' | 'full-id' | 'inherit';

/**
 * Which form a `model` value is: one of the aliases `sonnet`, `opus`, `haiku`, `fable`; `inherit`; or
 * anything else, a full model ID that "accepts the same values as the `--model` flag" (so the kit
 * does not judge whether such an ID exists).
 */
export const modelValueKind = (value: string): ModelValueKind => call('modelValueKind', value);
