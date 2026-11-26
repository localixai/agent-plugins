//! Port of `src/marketplace/header-policy.ts`, switched.
//!
//! Whether a `headersHelper` command may run, and which rules follow from where it was declared
//! (plugins/host-marketplace.md, "Authenticate archive downloads": the table of places; "When Claude
//! Code skips a headersHelper command or drops its output"; "How users accept a headersHelper
//! command"; "Installs and updates that refuse a command instead of asking"; "When a marketplace
//! `url` source's command runs"; settings-reference.md, `extraKnownMarketplaces`,
//! `allowManagedHooksOnly`, `disableCommandPluginSources`). Pure: the host's answers (trust, user
//! acceptance, policy) come in as data; the acceptance prompt itself is the host's.

use serde::{Deserialize, Serialize};

use crate::js::url;

mod facts;

pub use facts::{
    AcceptedCommand, HEADER_ORIGINS, HEADER_PLACES, HeaderLayer, HeaderOrigin, HeaderPlace,
    HeadersSetting, PLUGIN_OPERATIONS, PluginOperation,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Consent {
    None,
    Never,
    FolderTrust,
    SettingsApproval,
}

struct OriginRule {
    /// "Header filter": routing and identity names dropped from an entry declared here.
    filters_routing: bool,
    /// "Variables Claude Code removes": credential variables stay out of a command declared here.
    strips_credentials: bool,
    consent: Consent,
}

const fn origin_rule(origin: HeaderOrigin) -> OriginRule {
    match origin {
        HeaderOrigin::Catalog => OriginRule {
            filters_routing: true,
            strips_credentials: true,
            consent: Consent::None,
        },
        HeaderOrigin::User | HeaderOrigin::CliSettings | HeaderOrigin::Managed => OriginRule {
            filters_routing: false,
            strips_credentials: false,
            consent: Consent::None,
        },
        // Judgment: server-managed settings are managed settings, so no filter and no removal.
        HeaderOrigin::ServerManaged => OriginRule {
            filters_routing: false,
            strips_credentials: false,
            consent: Consent::SettingsApproval,
        },
        HeaderOrigin::Project => OriginRule {
            filters_routing: true,
            strips_credentials: true,
            consent: Consent::FolderTrust,
        },
        HeaderOrigin::AddDir => OriginRule {
            filters_routing: true,
            strips_credentials: true,
            consent: Consent::Never,
        },
    }
}

/// Whether a place filters at all: only an entry's headers are filtered, never a `url` source's.
const fn place_filters(place: HeaderPlace) -> bool {
    match place {
        HeaderPlace::UrlSource => false,
        HeaderPlace::Entry => true,
    }
}

/// "Every `marketplace.json` entry is filtered this way", and an entry in project or `--add-dir` settings.
#[must_use]
pub const fn filters_routing(layer: &HeaderLayer) -> bool {
    place_filters(layer.place) && origin_rule(layer.origin).filters_routing
}

/// Whether credential variables stay out of the command.
#[must_use]
pub const fn strips_credentials(layer: &HeaderLayer) -> bool {
    origin_rule(layer.origin).strips_credentials
}

/// Why a command did not run. `skip-helper`: the fixed `headers` still go (judgment where the docs
/// only say the command does not run). `refuse-download`: the plugin is not downloaded at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelperReason {
    /// `ignored-add-dir`.
    #[serde(rename = "ignored-add-dir")]
    IgnoredAddDir,
    /// `not-https`.
    #[serde(rename = "not-https")]
    NotHttps,
    /// `folder-not-trusted`.
    #[serde(rename = "folder-not-trusted")]
    FolderNotTrusted,
    /// `settings-not-approved`.
    #[serde(rename = "settings-not-approved")]
    SettingsNotApproved,
    /// `blocked-by-policy`.
    #[serde(rename = "blocked-by-policy")]
    BlockedByPolicy,
    /// `not-accepted`.
    #[serde(rename = "not-accepted")]
    NotAccepted,
    /// `needs-own-view`.
    #[serde(rename = "needs-own-view")]
    NeedsOwnView,
    /// `listed-in-errors`.
    #[serde(rename = "listed-in-errors")]
    ListedInErrors,
}

/// Why a command did not run.
pub const HELPER_REASONS: [HelperReason; 8] = [
    HelperReason::IgnoredAddDir,
    HelperReason::NotHttps,
    HelperReason::FolderNotTrusted,
    HelperReason::SettingsNotApproved,
    HelperReason::BlockedByPolicy,
    HelperReason::NotAccepted,
    HelperReason::NeedsOwnView,
    HelperReason::ListedInErrors,
];

/// What a refused command does to the download.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelperEffect {
    /// `skip-helper`.
    #[serde(rename = "skip-helper")]
    SkipHelper,
    /// `refuse-download`.
    #[serde(rename = "refuse-download")]
    RefuseDownload,
}

struct Refusal {
    effect: HelperEffect,
    message: &'static str,
}

const fn refusal(reason: HelperReason) -> Refusal {
    match reason {
        HelperReason::IgnoredAddDir => Refusal {
            effect: HelperEffect::SkipHelper,
            message: "A headersHelper set in the settings of a directory added with --add-dir is ignored; only that file's headers are sent",
        },
        HelperReason::NotHttps => Refusal {
            effect: HelperEffect::SkipHelper,
            message: "The marketplace URL does not start with https://, so its headersHelper does not run; requests carry only its headers field",
        },
        HelperReason::FolderNotTrusted => Refusal {
            effect: HelperEffect::SkipHelper,
            message: "The headersHelper is declared in a project settings file and runs only after the folder itself is trusted",
        },
        HelperReason::SettingsNotApproved => Refusal {
            effect: HelperEffect::SkipHelper,
            message: "The headersHelper is declared in server-managed settings and runs only after the user approves them",
        },
        HelperReason::BlockedByPolicy => Refusal {
            effect: HelperEffect::SkipHelper,
            message: "Managed settings block headersHelper commands (disableCommandPluginSources, or allowManagedHooksOnly without an explicit false)",
        },
        HelperReason::NotAccepted => Refusal {
            effect: HelperEffect::RefuseDownload,
            message: "The plugin runs a headersHelper command and the user has not accepted it, so its archive is not downloaded",
        },
        HelperReason::NeedsOwnView => Refusal {
            effect: HelperEffect::RefuseDownload,
            message: "The plugin runs a headersHelper command: install it by itself from its own view in /plugin",
        },
        HelperReason::ListedInErrors => Refusal {
            effect: HelperEffect::RefuseDownload,
            message: "The plugin runs a headersHelper command: it is listed in the /plugin Errors tab, install or update it yourself",
        },
    }
}

/// Whether the command may run, and if not, why and what that does to the download.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum HelperVerdict {
    /// `{ allowed: true }`.
    Allowed {
        /// Always true.
        allowed: bool,
    },
    /// `{ allowed: false, reason, effect, message }`.
    Refused {
        /// Always false.
        allowed: bool,
        /// Why.
        reason: HelperReason,
        /// What it does to the download.
        effect: HelperEffect,
        /// The words.
        message: String,
    },
}

type Check = fn(&HeaderLayer, &HeadersSetting) -> Option<HelperReason>;

fn consent_check(layer: &HeaderLayer, setting: &HeadersSetting) -> Option<HelperReason> {
    match origin_rule(layer.origin).consent {
        Consent::None => None,
        Consent::Never => Some(HelperReason::IgnoredAddDir),
        Consent::FolderTrust => (!setting.folder_trusted).then_some(HelperReason::FolderNotTrusted),
        Consent::SettingsApproval => {
            (!setting.settings_approved).then_some(HelperReason::SettingsNotApproved)
        }
    }
}

/// `/^https:\/\//i`.
fn https_check(layer: &HeaderLayer, _: &HeadersSetting) -> Option<HelperReason> {
    let https = layer.marketplace_url.as_deref().is_some_and(|url| {
        url.get(..8)
            .is_some_and(|start| start.eq_ignore_ascii_case("https://"))
    });
    if https {
        None
    } else {
        Some(HelperReason::NotHttps)
    }
}

/// What each operation does to an entry's command: only a single install or update asks the user.
const fn operation_reason(operation: PluginOperation) -> Option<HelperReason> {
    match operation {
        PluginOperation::Install | PluginOperation::Update => None,
        PluginOperation::BulkInstall
        | PluginOperation::Suggestion
        | PluginOperation::Dependency => Some(HelperReason::NeedsOwnView),
        PluginOperation::BackgroundUpdate | PluginOperation::SessionStart => {
            Some(HelperReason::ListedInErrors)
        }
    }
}

fn operation_check(layer: &HeaderLayer, _: &HeadersSetting) -> Option<HelperReason> {
    layer
        .operation
        .map_or(Some(HelperReason::NotAccepted), operation_reason)
}

/// The URL without its query string (and what follows it), or the text as written when it is no URL.
fn without_query(text: &str) -> String {
    let Some(parsed) = url::parse(text) else {
        return text.to_owned();
    };
    let (before, fragment) = parsed
        .href
        .find('#')
        .map_or((parsed.href.as_str(), ""), |at| parsed.href.split_at(at));
    let path = before.find('?').map_or(before, |at| before.split_at(at).0);
    format!("{path}{fragment}")
}

/// "Claude Code runs only the command it showed, for the archive URL it showed."
fn accepted_check(layer: &HeaderLayer, _: &HeadersSetting) -> Option<HelperReason> {
    let Some(accepted) = layer.accepted.as_ref() else {
        return Some(HelperReason::NotAccepted);
    };
    if layer.headers_helper.as_deref() != Some(accepted.command.as_str()) {
        return Some(HelperReason::NotAccepted);
    }
    let (shown, now) = (
        accepted.archive_url.as_str(),
        layer.archive_url.as_deref().unwrap_or(""),
    );
    let same = if accepted.query_counts == Some(true) {
        shown == now
    } else {
        without_query(shown) == without_query(now)
    };
    if same {
        None
    } else {
        Some(HelperReason::NotAccepted)
    }
}

/// "disableCommandPluginSources to true blocks headersHelper commands, and allowManagedHooksOnly
/// blocks them too unless disableCommandPluginSources is explicitly false. Under either block,
/// Claude Code still runs the command for a marketplace that managed settings themselves declare."
fn policy_check(_: &HeaderLayer, setting: &HeadersSetting) -> Option<HelperReason> {
    let (disabled, managed_only) = (
        setting.disable_command_plugin_sources,
        setting.allow_managed_hooks_only,
    );
    let blocked = disabled == Some(true) || (managed_only == Some(true) && disabled != Some(false));
    (blocked && !setting.managed_marketplace).then_some(HelperReason::BlockedByPolicy)
}

const fn place_checks(place: HeaderPlace) -> &'static [Check] {
    match place {
        HeaderPlace::UrlSource => &[https_check],
        HeaderPlace::Entry => &[operation_check, accepted_check],
    }
}

fn verdict_of(reason: HelperReason) -> HelperVerdict {
    let refused = refusal(reason);
    HelperVerdict::Refused {
        allowed: false,
        reason,
        effect: refused.effect,
        message: refused.message.to_owned(),
    }
}

/// Whether the layer's command may run now, or why not (and what that does to the download).
#[must_use]
pub fn helper_allowed(layer: &HeaderLayer, setting: &HeadersSetting) -> HelperVerdict {
    let checks = std::iter::once::<Check>(consent_check)
        .chain(place_checks(layer.place).iter().copied())
        .chain(std::iter::once::<Check>(policy_check));
    for check in checks {
        if let Some(reason) = check(layer, setting) {
            return verdict_of(reason);
        }
    }
    HelperVerdict::Allowed { allowed: true }
}

crate::registry::exports! {
    "marketplace/header-policy" {
        "HEADER_PLACES" => ok || HEADER_PLACES;
        "HEADER_ORIGINS" => ok || HEADER_ORIGINS;
        "PLUGIN_OPERATIONS" => ok || PLUGIN_OPERATIONS;
        "filtersRouting" => ok |layer: HeaderLayer| filters_routing(&layer);
        "stripsCredentials" => ok |layer: HeaderLayer| strips_credentials(&layer);
        "HELPER_REASONS" => ok || HELPER_REASONS;
        "helperAllowed" => ok |layer: HeaderLayer, setting: HeadersSetting| helper_allowed(&layer, &setting);
    }
}
