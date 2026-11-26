//! Port of `src/rules/restricts.ts`, switched.

use indexmap::IndexMap;

use crate::rules::types::PermissionBehavior;

/// Whether a rule of each behavior restricts what runs (deny, ask) rather than grants it (allow).
/// The docs widen what a restricting rule reaches and narrow what a granting one does, each time for
/// the same reason, the safe side: a deny or ask rule matches past any leading assignment, guards an
/// exact path a glob cannot express, covers what the kit cannot read, reaches the real directory of a
/// symlink, and so on. Each place that differs by behavior reads this one table.
#[must_use]
pub const fn restricts(behavior: PermissionBehavior) -> bool {
    match behavior {
        PermissionBehavior::Allow => false,
        PermissionBehavior::Deny | PermissionBehavior::Ask => true,
    }
}

/// `RESTRICTS` as the TypeScript's object: each behavior by its name and `restricts` of it, in its
/// order.
fn table() -> IndexMap<&'static str, bool> {
    [
        ("allow", PermissionBehavior::Allow),
        ("deny", PermissionBehavior::Deny),
        ("ask", PermissionBehavior::Ask),
    ]
    .into_iter()
    .map(|(name, behavior)| (name, restricts(behavior)))
    .collect()
}

crate::registry::exports! {
    "rules/restricts" {
        "RESTRICTS" => ok table;
    }
}
