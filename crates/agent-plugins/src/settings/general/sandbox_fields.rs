//! Ported from `src/settings/general/sandbox-fields.ts`; only the core uses it now, so it has no wrapper.
//!
//! What an invalid Boolean inside a managed `sandbox` block reads as (managed-settings.md,
//! "Invalid values inside `sandbox`"): a key that turns the sandbox or one of its restrictions on
//! counts as `true`, a key that loosens it as `false`, and `failIfUnavailable` is dropped "rather
//! than treating it as `true`". The docs name `enabled`, `network.allowManagedDomainsOnly` and
//! `allowUnsandboxedCommands`; the stance of every other key is a judgment (spec/judgments.md).

/// What an invalid Boolean reads as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    /// `true`.
    Restricts,
    /// `false`.
    Loosens,
    /// No value.
    Dropped,
}

/// The stance of every Boolean key of `sandbox`, by its dotted path.
pub const BOOLEAN_STANCES: [(&str, Stance); 15] = [
    ("enabled", Stance::Restricts),
    ("failIfUnavailable", Stance::Dropped),
    ("autoAllowBashIfSandboxed", Stance::Loosens),
    ("allowUnsandboxedCommands", Stance::Loosens),
    ("network.strictAllowlist", Stance::Restricts),
    ("network.allowManagedDomainsOnly", Stance::Restricts),
    ("network.allowAllUnixSockets", Stance::Loosens),
    ("network.allowLocalBinding", Stance::Loosens),
    ("filesystem.allowManagedReadPathsOnly", Stance::Restricts),
    ("filesystem.disabled", Stance::Loosens),
    ("credentials.allowPlaintextInject", Stance::Loosens),
    ("enableWeakerNestedSandbox", Stance::Loosens),
    ("allowNestedUserNamespaces", Stance::Loosens),
    ("enableWeakerNetworkIsolation", Stance::Loosens),
    ("allowAppleEvents", Stance::Loosens),
];

/// The value an invalid Boolean reads as, per stance; `dropped` has none.
#[must_use]
pub const fn stance_value(stance: Stance) -> Option<bool> {
    match stance {
        Stance::Restricts => Some(true),
        Stance::Loosens => Some(false),
        Stance::Dropped => None,
    }
}

/// The lists outside `credentials` (the docs name `excludedCommands` and
/// `network.allowedDomains`).
pub const SANDBOX_LISTS: [&str; 9] = [
    "excludedCommands",
    "network.allowedDomains",
    "network.deniedDomains",
    "network.allowUnixSockets",
    "network.allowMachLookup",
    "filesystem.allowWrite",
    "filesystem.denyWrite",
    "filesystem.denyRead",
    "filesystem.allowRead",
];

/// While a deny list, or any entry in it, is invalid, the allow lists it restrains are withheld "so
/// the managed allowlist grants nothing until you fix the deny list".
pub const WITHHELD_BY: [(&str, &[&str]); 3] = [
    ("network.deniedDomains", &["network.allowedDomains"]),
    (
        "filesystem.denyRead",
        &["filesystem.allowRead", "filesystem.allowWrite"],
    ),
    (
        "filesystem.denyWrite",
        &["filesystem.allowRead", "filesystem.allowWrite"],
    ),
];
