//! Port of `src/settings/plugins/mod-order.ts`, switched.
//!
//! The order the organization's and the user's own mods run in (settings-reference.md,
//! `prependPlugins` and `appendPlugins`).

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::{Port, Set};
use crate::settings::layers::PolicyScope;
use crate::settings::plugins::enabled_plugins::force_enabled_by_managed;
use crate::settings::plugins::plugin_id::{
    GUARD_ID_FORMS, is_guard_plugin_id, parse_plugin_id, spellings_of,
};
use crate::settings::view::PolicyView;

/// The two machine facts the ordering keys depend on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModOrderContext {
    /// Any admin-controlled source of settings is present on the machine.
    pub has_managed_settings: bool,
    /// How the session signs in: `claudeaiTeamEnterprise` only for a Team or Enterprise `/login`,
    /// not for a long-lived token (`PolicyAccount`).
    pub account: String,
}

/// An id a list names that takes no effect, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkippedPolicyMod {
    /// The id.
    pub id: String,
    /// Why.
    pub reason: String,
}

/// Whether the built-in guard loads, and where it sits (`GuardPlacement`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Guard {
    /// It loads.
    pub loads: bool,
    /// `listed`, `outermost` or `absent`.
    pub placement: &'static str,
}

/// The order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModOrderResult {
    /// Run before every mod a user installs, in this order. The built-in guard is here when
    /// listed.
    pub prepend: Vec<String>,
    /// Run after every mod a user installs, in this order.
    pub append: Vec<String>,
    /// The organization's mods that no list names: they run right after `prepend` and before every
    /// mod a user installs, in the order `orgMods` holds them (plugins/mods/events.md, "The order
    /// mods run in").
    pub unlisted: Vec<String>,
    /// The ids that take no effect.
    pub skipped: Vec<SkippedPolicyMod>,
    /// The built-in guard.
    pub guard: Guard,
}

/// Where a list comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListOrigin {
    Managed,
    User,
}

/// What every id of one list is judged against.
struct Request<'a> {
    view: &'a dyn PolicyView,
    key: &'static str,
    org_mods: &'a [String],
}

struct Listing {
    from: ListOrigin,
    ids: Vec<String>,
}

/// The text of the line Claude Code's debug log writes when a managed list names a plugin it
/// skips.
fn not_managed_mod(key: &str, id: &str) -> String {
    format!("{key} names {id}, which is not an enabled managed plugin with a hooks module; skipped")
}

/// The list a key holds once the source rule is applied: managed settings always; user settings
/// only on a machine with no managed settings and no Team or Enterprise `/login`; project, local
/// and `--settings` never (the key's scope line already drops those).
async fn listing_of(view: &dyn PolicyView, key: &str, user_may_order: bool) -> Option<Listing> {
    let holder = view.each_from_highest(key).await.into_iter().find(|each| {
        each.scope == PolicyScope::Managed || (each.scope == PolicyScope::User && user_may_order)
    })?;
    let ids = if let JsValue::Array(ids) = &holder.value {
        ids.iter().map(crate::js::array_join::string_of).collect()
    } else {
        Vec::new()
    };
    Some(Listing {
        from: if holder.scope == PolicyScope::Managed {
            ListOrigin::Managed
        } else {
            ListOrigin::User
        },
        ids,
    })
}

/// The id with the guard's two spellings made one, so a repeat of either is a repeat.
fn canonical(id: &str) -> String {
    spellings_of(&[GUARD_ID_FORMS.map(str::to_owned)], id)
        .into_iter()
        .next()
        .unwrap_or_else(|| id.to_owned())
}

/// Why an id of a managed list is skipped, or `None` when it takes effect.
async fn managed_skip(request: &Request<'_>, id: &str) -> Option<String> {
    if request.key == "prependPlugins" && is_guard_plugin_id(id) {
        return None;
    }
    if force_enabled_by_managed(request.view, id).await
        && request.org_mods.iter().any(|one| one == id)
    {
        None
    } else {
        Some(not_managed_mod(request.key, id))
    }
}

/// The ids of one list that take effect, in the listed order, each once, and those that do not.
async fn accept(
    request: &Request<'_>,
    listing: Option<&Listing>,
) -> (Vec<String>, Vec<SkippedPolicyMod>) {
    let mut taken = Vec::new();
    let mut skipped = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let from = listing.map_or(ListOrigin::User, |listing| listing.from);
    for id in listing
        .map(|listing| listing.ids.as_slice())
        .unwrap_or_default()
    {
        let one = canonical(id);
        if seen.contains(&one) {
            continue;
        }
        seen.push(one);
        match skip_reason(request, from, id).await {
            None => taken.push(id.clone()),
            Some(reason) => skipped.push(SkippedPolicyMod {
                id: id.clone(),
                reason,
            }),
        }
    }
    (taken, skipped)
}

async fn skip_reason(request: &Request<'_>, from: ListOrigin, id: &str) -> Option<String> {
    let key = request.key;
    if parse_plugin_id(id).is_none() {
        return Some(format!(
            "{key} names {id}, which is not a plugin-name@marketplace-name id; skipped"
        ));
    }
    if from == ListOrigin::Managed {
        return managed_skip(request, id).await;
    }
    // A user's list orders the user's own mods; it neither adds nor removes the built-in guard.
    is_guard_plugin_id(id)
        .then(|| format!("{key} cannot add the built-in guard from user settings; skipped"))
}

/// The order the organization's and the user's own mods run in (settings-reference.md,
/// `prependPlugins` and `appendPlugins`; plugins/mods/admin.md, "Install your organization's mods
/// and set the order"). `orgMods` holds the plugins the host has checked to meet the two
/// conditions the settings cannot show: the marketplace is a directory on the machine and lists the
/// plugin by a relative path, so it loads in place. The kit adds the third, a managed
/// `enabledPlugins` `true`.
///
/// Judgment (spec/judgments.md): the organization's mods that no list names are `unlisted`, in the
/// order of `orgMods`, a repeated id keeps its first place, the two spellings of the guard are one
/// id, text that is no plugin id and a guard in a user list are skipped with a reason, and a
/// managed layer counts as managed settings whatever the host says.
pub async fn mod_order(
    view: &dyn PolicyView,
    context: &ModOrderContext,
    org_mods: &[String],
) -> ModOrderResult {
    let has_managed = context.has_managed_settings
        || view
            .layers()
            .await
            .iter()
            .any(|layer| layer.scope == PolicyScope::Managed);
    let team_login = context.account == "claudeaiTeamEnterprise";
    let user_may_order = !has_managed && !team_login;

    let prepend_listing = listing_of(view, "prependPlugins", user_may_order).await;
    let prepend_request = Request {
        view,
        key: "prependPlugins",
        org_mods,
    };
    let (prepend, mut skipped) = accept(&prepend_request, prepend_listing.as_ref()).await;
    let append_request = Request {
        view,
        key: "appendPlugins",
        org_mods,
    };
    let append_listing = listing_of(view, "appendPlugins", user_may_order).await;
    let (append, append_skipped) = accept(&append_request, append_listing.as_ref()).await;
    // "An id listed in both `prependPlugins` and `appendPlugins` is prepended."
    let prepended: Vec<String> = prepend.iter().map(|id| canonical(id)).collect();
    let appended: Vec<String> = append
        .into_iter()
        .filter(|id| !prepended.contains(&canonical(id)))
        .collect();
    let mut listed = prepended;
    listed.extend(appended.iter().cloned());
    // "A mod that counts as yours runs before users' mods even when you list it nowhere."
    let mut unlisted = Vec::new();
    for id in org_mods {
        if force_enabled_by_managed(view, id).await && !listed.contains(&canonical(id)) {
            unlisted.push(id.clone());
        }
    }
    skipped.extend(append_skipped);
    let guard = guard_of(
        prepend_listing.as_ref(),
        &prepend,
        has_managed || team_login,
    );
    ModOrderResult {
        prepend,
        append: appended,
        unlisted,
        skipped,
        guard,
    }
}

/// "If managed `prependPlugins` is set, the guard loads only when that list names it, at the
/// position listed"; otherwise it loads on a machine with managed settings or a Team or
/// Enterprise `/login`, outermost (plugins/mods/reference.md; admin.md, "Know what happens by
/// default").
fn guard_of(
    prepend_listing: Option<&Listing>,
    prepended: &[String],
    loads_by_default: bool,
) -> Guard {
    let absent = Guard {
        loads: false,
        placement: "absent",
    };
    if prepend_listing.is_some_and(|listing| listing.from == ListOrigin::Managed) {
        return if prepended.iter().any(|id| is_guard_plugin_id(id)) {
            Guard {
                loads: true,
                placement: "listed",
            }
        } else {
            absent
        };
    }
    if loads_by_default {
        Guard {
            loads: true,
            placement: "outermost",
        }
    } else {
        absent
    }
}

crate::registry::exports! {
    "settings/plugins/mod-order" {
        "modOrder" => async ok |view: Port, context: ModOrderContext, org_mods: Set<String>|
            mod_order(&view, &context, &org_mods.members).await;
    }
}
