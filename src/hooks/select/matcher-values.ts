/**
 * The values a matcher can name, per event, as hooks.md lists them in each event's section and in
 * "Matcher patterns". Where the SDK types the field as a union, the list is asserted equal to it, so
 * an SDK release that adds a value fails the typecheck here.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/matcher_values.rs`, docs/RUST.md).
 */
import type { HookInputOf } from '../../protocol/hook-events.ts';
import type { Assert, Equal } from '../../protocol/type-equality.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/select/matcher-values');

/** The documented matcher values of the events that have a fixed set of them. */
export const MATCHER_VALUES: {
  readonly SessionStart: readonly ['startup', 'resume', 'clear', 'compact', 'fork'];
  readonly Setup: readonly ['init', 'maintenance'];
  readonly SessionEnd: readonly ['clear', 'resume', 'logout', 'prompt_input_exit', 'other'];
  readonly Notification: readonly [
    'permission_prompt',
    'idle_prompt',
    'auth_success',
    'auth_storage_failure',
    'elicitation_dialog',
    'elicitation_url_dialog',
    'elicitation_complete',
    'elicitation_response',
    'agent_needs_input',
    'agent_completed',
    'quota_auto_resume_fired',
    'quota_auto_resume_stale',
    'quota_auto_resume_disabled',
    'plugin_notification',
  ];
  readonly PreCompact: readonly ['manual', 'auto'];
  readonly PostCompact: readonly ['manual', 'auto'];
  readonly ConfigChange: readonly [
    'user_settings',
    'project_settings',
    'local_settings',
    'policy_settings',
    'skills',
  ];
  readonly DirectoryAdded: readonly ['slash_command', 'register_repo_root'];
  readonly InstructionsLoaded: readonly [
    'session_start',
    'nested_traversal',
    'path_glob_match',
    'include',
    'compact',
  ];
  readonly StopFailure: readonly [
    'rate_limit',
    'overloaded',
    'authentication_failed',
    'oauth_org_not_allowed',
    'account_on_hold',
    'billing_error',
    'invalid_request',
    'model_not_found',
    'server_error',
    'max_output_tokens',
    'cloud_credential_error',
    'unknown',
  ];
} = call('MATCHER_VALUES');

/** @public Drift guard. */
export type SessionStartSourcesMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.SessionStart)[number], HookInputOf<'SessionStart'>['source']>
>;
/** @public Drift guard. */
export type SetupTriggersMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.Setup)[number], HookInputOf<'Setup'>['trigger']>
>;
/** @public Drift guard. */
export type SessionEndReasonsMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.SessionEnd)[number], HookInputOf<'SessionEnd'>['reason']>
>;
/** @public Drift guard. */
export type PreCompactTriggersMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.PreCompact)[number], HookInputOf<'PreCompact'>['trigger']>
>;
/** @public Drift guard. */
export type PostCompactTriggersMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.PostCompact)[number], HookInputOf<'PostCompact'>['trigger']>
>;
/** @public Drift guard. */
export type ConfigChangeSourcesMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.ConfigChange)[number], HookInputOf<'ConfigChange'>['source']>
>;
/** @public Drift guard. */
export type DirectoryAddedSourcesMatchSdk = Assert<
  Equal<(typeof MATCHER_VALUES.DirectoryAdded)[number], HookInputOf<'DirectoryAdded'>['source']>
>;
/** @public Drift guard. */
export type InstructionsLoadReasonsMatchSdk = Assert<
  Equal<
    (typeof MATCHER_VALUES.InstructionsLoaded)[number],
    HookInputOf<'InstructionsLoaded'>['load_reason']
  >
>;
/** @public Drift guard: every documented error is one the SDK types. */
export type StopFailureErrorsInSdk = Assert<
  (typeof MATCHER_VALUES.StopFailure)[number] extends HookInputOf<'StopFailure'>['error']
    ? true
    : false
>;
