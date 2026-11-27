/**
 * Where a host's settings come from, read into layers: each source the host has, parsed, with the
 * managed tier composed from every managed source when the host says what each holds.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/sources.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PolicyScope, PolicyLayer } from './layers.ts';
import type { ManagedReads } from './managed/load.ts';

const call = coreModule('settings/sources');

/**
 * Where a host's settings come from. The kit knows no file path, MDM profile, registry value or
 * server-managed fetch: the host reads each source it has and hands over its raw JSON, either
 * parsed or as text. Choosing among managed sources (`managedSourcesBehavior`, `policyHelper`) is
 * the host's, not the kit's. Optional: a host may build layers with `readSettingsLayer` itself.
 */
export interface SettingsSources {
  /** The source's JSON (an object, or its text); `undefined` when the source does not exist. */
  read(scope: PolicyScope, signal: AbortSignal): Promise<unknown>;
  /**
   * What names the source in a report: the file's path, `per-user managed preferences`,
   * `Registry: HKLM\SOFTWARE\Policies\ClaudeCode\Settings` (errors.md, "Managed settings document
   * could not be parsed"). Without it a report names no source.
   */
  label?(scope: PolicyScope): string | undefined;
  /**
   * What a machine holds at every managed source (remote settings, MDM policy, the managed files, the
   * HKCU registry). When the host has it, the kit chooses among them and composes the managed tier
   * (`managedSourcesBehavior`, managed-settings.md, "How Claude Code combines managed sources") and
   * `read('managed')` is not asked.
   */
  managed?(signal: AbortSignal): Promise<ManagedReads>;
}

/** What loading every source came to. */
export interface LoadedSettings {
  /** The sources that exist and could be read, highest first; none when `fatal`. */
  readonly layers: readonly PolicyLayer[];
  readonly issues: readonly PluginIssue[];
  /**
   * A managed document that is present and does not parse as a JSON object: Claude Code "exits with
   * code 1 at startup instead of running without the policy the document carries" (errors.md). The
   * host stops and reports `issues`; no layer is returned, so none runs without the policy.
   */
  readonly fatal: boolean;
}

/**
 * Reads every scope the host has, in precedence order, into layers. A broken user, project, local or
 * `--settings` source is skipped; a broken managed one is `fatal`. A host that says what each managed
 * source holds (`SettingsSources.managed`) gets the managed tier composed by the kit.
 */
export const loadSettingsLayers = (
  sources: SettingsSources,
  options: { readonly signal: AbortSignal; readonly scopes?: readonly PolicyScope[] },
): Promise<LoadedSettings> => call.async('loadSettingsLayers', sources, options);
