//! Port of `src/hooks/select/matcher-values.ts`, switched.
//!
//! The values a matcher can name, per event, as hooks.md lists them in each event's section and in
//! "Matcher patterns". Where the SDK types the field as a union, the TypeScript asserts the list
//! equal to it, so an SDK release that adds a value fails the typecheck there (the drift guards are
//! types and stay TypeScript).

use indexmap::IndexMap;

const SESSION_START_SOURCES: [&str; 5] = ["startup", "resume", "clear", "compact", "fork"];
const SETUP_TRIGGERS: [&str; 2] = ["init", "maintenance"];
/// `bypass_permissions_disabled` is documented as removed in v2.1.234: "Drop it from your matchers".
const SESSION_END_REASONS: [&str; 5] = ["clear", "resume", "logout", "prompt_input_exit", "other"];
const COMPACT_TRIGGERS: [&str; 2] = ["manual", "auto"];
const CONFIG_CHANGE_SOURCES: [&str; 5] = [
    "user_settings",
    "project_settings",
    "local_settings",
    "policy_settings",
    "skills",
];
const DIRECTORY_ADDED_SOURCES: [&str; 2] = ["slash_command", "register_repo_root"];
const INSTRUCTIONS_LOAD_REASONS: [&str; 5] = [
    "session_start",
    "nested_traversal",
    "path_glob_match",
    "include",
    "compact",
];
/// The SDK types `notification_type` as a plain string; these are the documented values.
const NOTIFICATION_TYPES: [&str; 14] = [
    "permission_prompt",
    "idle_prompt",
    "auth_success",
    "auth_storage_failure",
    "elicitation_dialog",
    "elicitation_url_dialog",
    "elicitation_complete",
    "elicitation_response",
    "agent_needs_input",
    "agent_completed",
    "quota_auto_resume_fired",
    "quota_auto_resume_stale",
    "quota_auto_resume_disabled",
    "plugin_notification",
];
/// The documented `StopFailure` error types. The SDK also has `verification_required`, which the
/// docs do not list, so this list is checked to be a subset of the SDK's union, not equal to it.
const STOP_FAILURE_ERRORS: [&str; 12] = [
    "rate_limit",
    "overloaded",
    "authentication_failed",
    "oauth_org_not_allowed",
    "account_on_hold",
    "billing_error",
    "invalid_request",
    "model_not_found",
    "server_error",
    "max_output_tokens",
    "cloud_credential_error",
    "unknown",
];

/// The documented matcher values of the events that have a fixed set of them.
#[must_use]
pub fn matcher_values() -> IndexMap<&'static str, &'static [&'static str]> {
    IndexMap::from([
        ("SessionStart", &SESSION_START_SOURCES[..]),
        ("Setup", &SETUP_TRIGGERS[..]),
        ("SessionEnd", &SESSION_END_REASONS[..]),
        ("Notification", &NOTIFICATION_TYPES[..]),
        ("PreCompact", &COMPACT_TRIGGERS[..]),
        ("PostCompact", &COMPACT_TRIGGERS[..]),
        ("ConfigChange", &CONFIG_CHANGE_SOURCES[..]),
        ("DirectoryAdded", &DIRECTORY_ADDED_SOURCES[..]),
        ("InstructionsLoaded", &INSTRUCTIONS_LOAD_REASONS[..]),
        ("StopFailure", &STOP_FAILURE_ERRORS[..]),
    ])
}

crate::registry::exports! {
    "hooks/select/matcher-values" {
        "MATCHER_VALUES" => ok matcher_values;
    }
}
