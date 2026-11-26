import type { ModelAlias } from './model-value.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What a `model` value is, who may write which kind, and what happens to a value the organization's
 * `availableModels` allowlist blocks (sub-agents.md, "Choose a model"). The order of the sources is
 * `resolveSubagentModel`'s; this file holds the rest of the section. Resolving an alias to a version
 * and reading the allowlist are the host's: it passes what the allowlist permits.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/model_choice.rs`, docs/RUST.md).
 */
const call = coreModule('agents/model-choice');

type Writer = 'frontmatter' | 'invocation';

/** Whether the `model` field of a definition, or the `model` parameter of an Agent call, may hold the value. */
export const modelValueAccepted = (writer: Writer, value: string): boolean =>
  call('modelValueAccepted', writer, value);

/** What the organization's `availableModels` allowlist says, as the host evaluates it. */
export interface ModelAllowlist {
  /** The model may be used. */
  permits(model: string): boolean;
  /** The newest version of a family the allowlist permits (the substitution rules of `/model`), or `undefined`. */
  newestPermitted(family: ModelAlias): string | undefined;
}

export interface CheckedModel {
  readonly model: string;
  /** What was asked for, when it was replaced: the interactive warning "naming the requested model and the model the subagent runs on". */
  readonly replaced?: { readonly requested: string; readonly running: string };
}

export interface ModelCheckInput {
  /** The model the order of `resolveSubagentModel` chose (an alias or a full ID). */
  readonly chosen: string;
  /** `CLAUDE_CODE_SUBAGENT_MODEL`, tried first when the chosen value is blocked. */
  readonly envModel?: string | undefined;
  /** The inherited model: the main conversation's. */
  readonly inherited: string;
  /** Absent: no allowlist, nothing is blocked. */
  readonly allowlist?: ModelAllowlist | undefined;
}

/**
 * Applies the allowlist to the model a subagent was about to run on. A blocked family alias such as
 * `opus` becomes the newest version of that family the allowlist permits; "for any other blocked
 * value ... or when the allowlist permits no version of the family" the subagent runs on the
 * inherited model, "If you set `CLAUDE_CODE_SUBAGENT_MODEL`, Claude Code tries that model first,
 * under these same rules". The inherited model is never checked: it is what the session already runs.
 */
export const checkAgainstAllowlist = (input: ModelCheckInput): CheckedModel =>
  call('checkAgainstAllowlist', input);
