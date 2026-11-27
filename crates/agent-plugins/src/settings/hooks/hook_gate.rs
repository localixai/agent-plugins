//! The types of `src/settings/hooks/hook-gate.ts` that the ported hook modules read: the three
//! decisions about hooks that the settings make, read once from the view. The module itself (the
//! locks and the verdict per origin) is not ported yet.

/// The three decisions about hooks that the settings make, read once from the view.
pub trait HookGate {
    /// `disableAllHooks` is `true` after precedence, in managed settings.
    fn managed_disable(&self) -> bool;
    /// `disableAllHooks` is `true` after precedence, in any other file, or the SDK's
    /// `managedSettings` carries it as `true`, which is a lock and not a ranked value.
    fn other_disable(&self) -> bool;
    /// `allowManagedHooksOnly` is `true`, or `managedSettings` carries `disableAllHooks: true`,
    /// which binds as it.
    fn managed_only(&self) -> bool;
    /// Whether managed settings force-enable the plugin with this whole `plugin@marketplace` id.
    fn force_enabled(&self, id: &str) -> bool;
}
