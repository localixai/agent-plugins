import type { PluginIssue } from '../format/issues.ts';
import type { PluginManifest } from '../format/manifest-schema.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * `claude plugin tag` (plugins/cli-reference.md § plugin tag): the release tag of a plugin, its
 * preview, and the git commands that create and push it, from what the host observed.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/tag_plan.rs`, docs/RUST.md).
 */
const call = coreModule('cli/tag-plan');

/** Host observations, after walking upward for the marketplace that lists this directory. */
export interface TagFacts {
  readonly directory: string;
  readonly manifest: PluginManifest;
  readonly validationIssues: readonly PluginIssue[];
  readonly marketplace?: { readonly file: string; readonly version?: string };
  readonly dirty: boolean;
  readonly tagExists: boolean;
}

export interface TagOptions {
  readonly dryRun?: boolean;
  readonly force?: boolean;
  readonly message?: string;
  readonly push?: boolean;
  readonly remote?: string;
}

export interface TagCommand {
  readonly command: 'git';
  readonly args: readonly string[];
  readonly cwd: string;
}

export interface TagPlan {
  readonly ready: boolean;
  readonly tag: string;
  readonly preview: readonly string[];
  readonly commands: readonly TagCommand[];
  readonly push: TagCommand;
  readonly remote: string;
  readonly dryRun: boolean;
  readonly pushes: boolean;
  readonly refusal?: string;
}

/** plugins/cli-reference.md § plugin tag. Commands are exec arguments, never interpolated shell text. */
export const planPluginTag = (options: TagOptions, facts: TagFacts): TagPlan =>
  call('planPluginTag', options, facts);

/** A failed push does not undo the tag already created locally. */
export const finishPluginTag = (
  plan: TagPlan,
  effects: { readonly created: boolean; readonly pushed?: boolean; readonly error?: string },
): { readonly exitCode: 0 | 1; readonly tagCreated: boolean; readonly lines: readonly string[] } =>
  call('finishPluginTag', plan, effects);
