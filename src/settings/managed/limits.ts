/**
 * What managed settings cannot enforce, and how to check a policy that does not behave
 * (managed-settings.md "What a developer can change"; plugins/org.md "Plan for what managed settings
 * can't enforce" and "Troubleshoot policy"). Pure: each function answers from facts the host gives.
 * The docs give the facts; the keys and the returned shape are the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/limits.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/limits');

export interface ManagedActor {
  /** An administrator on the machine can edit the managed source itself. */
  readonly localAdmin?: boolean;
  /** The developer calls the API from a tool other than Claude Code. */
  readonly otherTool?: boolean;
  /** The edit is to the local copy of server-managed settings. */
  readonly editsServerCache?: boolean;
}

export interface ManagedBinding {
  readonly bound: boolean;
  /** Why not, or the remedy the docs name. */
  readonly note?: string;
}

export const managedBinding = (actor: ManagedActor): ManagedBinding =>
  call('managedBinding', actor);

export const UNENFORCEABLE_REQUESTS: readonly [
  'per-user-targeting',
  'entries-inside-allowed-marketplace',
  'hide-plugin-command',
  'gate-plugin-dir-through-allowlist',
  'enforce-claude-ai-toggles',
] = call('UNENFORCEABLE_REQUESTS');

export type UnenforceableRequest = (typeof UNENFORCEABLE_REQUESTS)[number];

export const NEAREST_CONTROLS: { readonly [R in UnenforceableRequest]: readonly string[] } =
  call('NEAREST_CONTROLS');

export const POLICY_SYMPTOMS: readonly [
  'managed-file-not-parsed',
  'managed-source-not-loaded',
  'blocked-by-enterprise-policy',
  'disabled-plugin-still-loads',
] = call('POLICY_SYMPTOMS');

export type PolicySymptom = (typeof POLICY_SYMPTOMS)[number];

export const SYMPTOM_CHECKS: { readonly [S in PolicySymptom]: string } = call('SYMPTOM_CHECKS');

export const managedSourceLoaded = (settingSourcesLine: string): boolean =>
  call('managedSourceLoaded', settingSourcesLine);

export type SkillsDirCause = 'ask-administrator' | 'check-repository-plugin-causes';

export const skillsDirCause = (settingSourcesLine: string): SkillsDirCause =>
  call('skillsDirCause', settingSourcesLine);

export interface WholePluginFacts {
  /** The folder is in a marketplace the organization allows and installed from there. */
  readonly allowedMarketplaceAvailable: boolean;
  /** The administrator turned off the `--plugin-dir` flag (`disableSideloadFlags`). */
  readonly sideloadFlagsDisabled: boolean;
}

export const wholePluginRoutes = (
  facts: WholePluginFacts,
): readonly ('marketplace' | 'plugin-dir')[] => call('wholePluginRoutes', facts);

export interface PayloadEntry {
  readonly key: string;
  readonly valid: boolean;
  /** The key fails closed: an invalid value falls back to a stricter one. */
  readonly fallsBackStricter?: boolean;
}

export interface PayloadOutcome {
  readonly apply: boolean;
  /** The cache is replaced only when the payload is applied. */
  readonly updateCache: boolean;
  /** The failed-load cause shown by the startup notice, `/status` and `claude doctor`. */
  readonly failure?: string;
}

export const deliveredPayloadOutcome = (entries: readonly PayloadEntry[]): PayloadOutcome =>
  call('deliveredPayloadOutcome', entries);

export const retentionValueApplies = (key: string, valid: boolean): boolean =>
  call('retentionValueApplies', key, valid);
