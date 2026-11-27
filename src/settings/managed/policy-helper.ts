/**
 * `policyHelper`: an executable that computes managed settings at startup (settings-reference.md,
 * `policyHelper`, "Write the helper output", "Helper failures", `policyHelper.path`,
 * `policyHelper.timeoutMs`, `policyHelper.refreshIntervalMs`; managed-settings.md, "Compute the
 * policy with a helper program"). This module judges the key and the helper's output; running the
 * process is `helper-run.ts`, through the command port.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/policy_helper.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/policy-helper');

export const HELPER_DEFAULT_TIMEOUT_MS: number = call('HELPER_DEFAULT_TIMEOUT_MS');
export const HELPER_MIN_TIMEOUT_MS: number = call('HELPER_MIN_TIMEOUT_MS');
export const HELPER_MIN_REFRESH_MS: number = call('HELPER_MIN_REFRESH_MS');
/** "Reads a JSON envelope from stdout, capped at 1 MiB"; a helper that writes more to stdout or to stderr fails. */
export const HELPER_OUTPUT_CAP_BYTES: number = call('HELPER_OUTPUT_CAP_BYTES');

/** The SDK's `policyHelper` with the timeout filled in: `refreshIntervalMs` stays unset until the settings say. */
type SdkPolicyHelper = NonNullable<Settings['policyHelper']>;
export type PolicyHelper = Required<Pick<SdkPolicyHelper, 'path' | 'timeoutMs'>> &
  Pick<SdkPolicyHelper, 'refreshIntervalMs'>;

export type HelperKey = { readonly helper: PolicyHelper } | { readonly issue: PluginIssue };

/**
 * Reads the key's value. An invalid value, such as a bare path or a `timeoutMs` below its minimum,
 * is "a dropped entry": the session starts on the remaining managed settings without a helper.
 */
export const readPolicyHelper = (value: unknown): HelperKey => call('readPolicyHelper', value);

/** The OS families `policyHelper.path` is judged for. */
export const PATH_FLAVORS: readonly ['posix', 'windows'] = call('PATH_FLAVORS');
export type PathFlavor = (typeof PATH_FLAVORS)[number];

/** Why `path` breaks the rules of `policyHelper.path`; a run with such a path fails. */
export const helperPathFault = (path: string, flavor: PathFlavor): string | undefined =>
  call('helperPathFault', path, flavor);

export type HelperOutput =
  | { readonly settings: Settings | undefined }
  | { readonly failure: string };

/**
 * What the helper's output comes to. A bare settings object with no `managedSettings` key "parses
 * with `managedSettings` undefined and applies nothing, and Claude Code reports no error". A schema
 * violation that survives the entry repairs fails the run: the helper's settings are held to a
 * stricter standard than a file's (managed-settings.md, "Find entries Claude Code dropped").
 */
export const readHelperOutput = (stdout: string, stderr: string): HelperOutput =>
  call('readHelperOutput', stdout, stderr);
