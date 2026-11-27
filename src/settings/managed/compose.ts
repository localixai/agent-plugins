/**
 * Which managed sources apply and the managed tier they make (managed-settings.md, "How Claude Code
 * combines managed sources", "Keys read from every admin source", "Compose every managed source";
 * settings-reference.md, `managedSourcesBehavior`). Pure over the documents the host read.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/compose.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyLayer } from '../layers.ts';
import type { AdminDocument, ManagedSource } from './admin-source.ts';
import type { InForceNote } from './cross-source.ts';
import type { PolicyHelper } from './policy-helper.ts';

const call = coreModule('settings/managed/compose');

export type ManagedBehavior = NonNullable<Settings['managedSourcesBehavior']>;
export const MANAGED_BEHAVIORS: readonly ManagedBehavior[] = call('MANAGED_BEHAVIORS');

export interface ComposeOptions {
  /**
   * The host passed parent settings that carry a restrictive key. HKCU "applies only when no admin
   * document is present above it and no host-supplied parent settings supply a restrictive key".
   */
  readonly parentRestrictive?: boolean;
}

/** What the policy helper does on this machine. */
export interface HelperPlan {
  /** The helper the highest source that carries a policy key names; absent when none, or its value is invalid. */
  readonly helper?: PolicyHelper;
  /** The helper runs at launch: its source is an MDM policy or a managed file. */
  readonly runsAtLaunch: boolean;
  /** Server-managed settings are the policy, so the helper waits for a fetch that reports them removed. */
  readonly shadowedByRemote: boolean;
}

export interface ManagedComposition {
  /** The managed tier: the layer to give the policy view. */
  readonly layer: PolicyLayer;
  readonly behavior: ManagedBehavior;
  /** The sources applied, highest first. */
  readonly used: readonly ManagedSource[];
  /** Sources found and not applied. */
  readonly skipped: readonly ManagedSource[];
  /** `/status` `Setting sources`: `Enterprise managed settings (file)`; absent when no source delivers a policy. */
  readonly settingSources: string | undefined;
  /** `/status` `Skipped sources`; absent when none was skipped. */
  readonly skippedSources: string | undefined;
  /** An administrator deploys managed settings here (`PolicyContext.hasManagedSettings`). */
  readonly hasManagedSettings: boolean;
  /** `parentSettingsBehavior` is `"merge"` in the highest-priority admin source (`PolicyContext.parentMerge`). */
  readonly parentMerge: boolean;
  readonly helper: HelperPlan;
  /** The repaired settings of every admin source that delivers a policy key, highest first. */
  readonly admins: readonly Settings[];
  /**
   * Some admin source sets `forceRemoteSettingsRefresh` to `true`. "The startup check runs before the
   * helper and reads any admin source", so a helper's output does not change it.
   */
  readonly forceRemoteRefresh: boolean;
  /** A lock or allowlist an admin source sets and another overrides. */
  readonly notInForce: readonly InForceNote[];
  readonly issues: readonly PluginIssue[];
  /** A present admin document cannot be parsed, or one cannot be read and no source supplies a policy: Claude Code exits. */
  readonly fatal: boolean;
}

export const composeManaged = (
  documents: readonly AdminDocument[],
  options: ComposeOptions = {},
): ManagedComposition => call('composeManaged', documents, options);

/**
 * The managed tier after a policy helper emitted `managedSettings` (settings-reference.md, "Write
 * the helper output"): "that object becomes the only managed settings source for the run", the
 * cross-source keys read from it alone, and parent settings are never merged.
 */
export const withHelperSettings = (
  composition: ManagedComposition,
  helperOutput: Settings,
): ManagedComposition => call('withHelperSettings', composition, helperOutput);
