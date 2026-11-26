//! Port of `src/format/marketplace/source-failures.ts`, switched.
//!
//! The two errors Claude Code gives for a marketplace source it parses and cannot load
//! (marketplace-reference.md, the `npm` row of "Marketplace sources" and "Source values valid only in
//! policy lists"), defined once for the load status, the notice and the fetch plan.

/// The error of an `npm` marketplace source.
pub const SOURCE_FAILURE_NPM: &str = "NPM marketplace sources not yet implemented";
/// The error of a source type only a policy list takes.
pub const SOURCE_FAILURE_POLICY_ONLY: &str = "Unsupported marketplace source type";

/// The notice for a type only a policy list takes, outside one.
#[must_use]
pub fn policy_only_notice(kind: &str) -> String {
    format!(
        "A {kind} source is valid only in strictKnownMarketplaces and blockedMarketplaces; elsewhere it fails to load: {SOURCE_FAILURE_POLICY_ONLY}"
    )
}

crate::registry::exports! {
    "format/marketplace/source-failures" {
        "SOURCE_FAILURE" => ok || crate::js::JsValue::from([("npm", SOURCE_FAILURE_NPM.into()), ("policyOnly", SOURCE_FAILURE_POLICY_ONLY.into())]);
        "policyOnlyNotice" => ok |kind: String| policy_only_notice(&kind);
    }
}
