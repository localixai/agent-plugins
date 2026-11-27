//! The types of `src/settings/value-spec.ts` (a module of types only, so it has no port): what the
//! kit knows about each policy key, one row per key, from its entry in settings-reference.md: the
//! files it may come from (the entry's **Scope** line), how files combine (**Scope** notes and
//! settings.md, "Lists merge instead of overriding"), and what to do with an invalid value
//! (managed-settings.md, "Keys that fail closed"). The resolvers of each area read their key
//! through the view and never repeat these facts.

use std::fmt;
use std::sync::LazyLock;

use crate::js::JsValue;
use crate::settings::general::exceptions::ManagedException;
use crate::settings::key_spec::ScopeRule;
use crate::settings::merge::Merge;

/// Where a row reads its key in a settings object.
pub type Read = Box<dyn Fn(&JsValue) -> JsValue + Send + Sync>;

/// Whether the admin tier alone counts, given the managed settings.
pub type PinnedWhen = Box<dyn Fn(&[JsValue]) -> bool + Send + Sync>;

/// One key's row.
pub struct ValueSpec {
    /// The files the key may come from.
    pub scopes: ScopeRule,
    /// How the files combine.
    pub merge: Merge,
    /// The key's managed value is not an override: a restrictive value of a lower scope stays (settings.md).
    pub exception: Option<&'static LazyLock<ManagedException>>,
    /// How the admin tier holds the key; absent, every scope combines by `merge`.
    pub admin: Option<AdminTier>,
    /// Where a key inside a block, or a key two spellings share, is read in a settings object; a
    /// top-level key needs none (`undefined` where it finds nothing).
    pub read: Option<Read>,
    /// The names down a block to a key inside it, so the merged settings carry the value this row decided.
    pub path: Option<&'static [&'static str]>,
}

/// A row with no exception, admin tier, reader or path: the shape most rows have.
#[must_use]
pub const fn spec(scopes: ScopeRule, merge: Merge) -> ValueSpec {
    ValueSpec {
        scopes,
        merge,
        exception: None,
        admin: None,
        read: None,
        path: None,
    }
}

/// How the admin tier holds a key (managed-settings.md, "Let an embedding host add policy"). The
/// admin tier is the highest managed source that sets the key, else the parent settings.
#[derive(Default)]
pub struct AdminTier {
    /// A managed value blocks the parent's; the other scopes still combine (`allowedMcpServers`).
    pub blocks_parent: Option<bool>,
    /// The admin tier's value applies alone when it sets one; no other scope adds to it (`availableModels`).
    pub owns: Option<bool>,
    /// While it holds for the managed settings, only the admin tier counts, set or not:
    /// `allowedMcpServers` under `allowManagedMcpServersOnly`, the sandbox allowlists under their
    /// `allowManaged*Only` locks, `sandbox.filesystem.disabled` (sandboxing.md, "Which settings can
    /// disable it").
    pub pinned_when: Option<PinnedWhen>,
}

impl fmt::Debug for ValueSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValueSpec")
            .field("scopes", &self.scopes)
            .field("merge", &self.merge.kind)
            .field("exception", &self.exception)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}
