//! Port of `src/updates/claude-code-version-policy.ts`, switched.
//!
//! The version policy of Claude Code itself (settings-reference.md, "Updates and versioning":
//! `autoUpdatesChannel`, `minimumVersion`, `requiredMinimumVersion`, `requiredMaximumVersion`;
//! setup.md, "Configure release channel" and "Pin a minimum version"). Pure over the settings layers
//! and the versions the host knows: the kit decides what an update may install and whether a start
//! is refused; the host installs and exits.
//!
//! The four keys are read here, not through `settings/key-spec.ts`: that table is closed over the
//! plugin, hook, skill, workflow and channel keys the settings requirements name, and its tests
//! count them (spec/judgments.md).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::format::versions::compare_versions;
use crate::js::JsValue;
use crate::marketplace::semver::normalized_version;
use crate::settings::key_spec::scope_carries;
use crate::settings::layers::{PolicyLayer, PolicyScope, ordered_layers};

/// The release channels.
pub const CHANNELS: [Channel; 2] = [Channel::Latest, Channel::Stable];

/// A release channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "each variant is the channel of its name")]
pub enum Channel {
    Latest,
    Stable,
}

impl Channel {
    const fn name(self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::Stable => "stable",
        }
    }
}

/// The policy the settings set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionPolicy {
    /// `autoUpdatesChannel`; unset (or a value the docs do not list) follows `latest`.
    pub channel: Channel,
    /// The highest valid `minimumVersion` of the files, normalized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<String>,
    /// Managed settings only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_minimum_version: Option<String>,
    /// Managed settings only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_maximum_version: Option<String>,
}

fn is_channel(value: &JsValue) -> Option<Channel> {
    CHANNELS
        .into_iter()
        .find(|one| value.as_str() == Some(one.name()))
}

/// "a value that isn't a valid version is ignored".
fn valid_version(value: Option<&JsValue>) -> Option<String> {
    value.and_then(JsValue::as_str).and_then(normalized_version)
}

/// The **Scope** line of the two required keys: "Managed. Claude Code gives no warning when it
/// ignores the key elsewhere."
const MANAGED_ONLY: [(PolicyScope, bool); 7] = [
    (PolicyScope::Managed, true),
    (PolicyScope::Parent, false),
    (PolicyScope::Flag, false),
    (PolicyScope::Local, false),
    (PolicyScope::Project, false),
    (PolicyScope::User, false),
    (PolicyScope::AddDir, false),
];

fn managed_version(layers: &[PolicyLayer], key: &str) -> Option<String> {
    layers.iter().find_map(|layer| {
        let managed = MANAGED_ONLY
            .iter()
            .any(|(scope, only)| *scope == layer.scope && *only);
        if managed {
            valid_version(layer.settings.get(key))
        } else {
            None
        }
    })
}

/// The higher of two versions.
fn higher(a: String, b: String) -> String {
    if compare_versions(&a, &b).unwrap_or(Ordering::Equal) == Ordering::Less {
        b
    } else {
        a
    }
}

/// Judgments (spec/judgments.md): the channel is the highest-precedence valid one; the minimum is
/// the highest valid `minimumVersion` of the files, so a managed floor "user and project settings
/// can't lower" holds while a user may raise it.
#[must_use]
pub fn read_version_policy(layers: &[PolicyLayer]) -> VersionPolicy {
    let ordered = ordered_layers(layers);
    // A `--add-dir` directory carries only `enabledPlugins` and `extraKnownMarketplaces` (permissions.md).
    let carrying = |key: &'static str| {
        ordered
            .iter()
            .filter(move |layer| scope_carries(layer.scope, key))
    };
    let channel = carrying("autoUpdatesChannel").find_map(|layer| {
        layer
            .settings
            .get("autoUpdatesChannel")
            .and_then(is_channel)
    });
    let minimum = carrying("minimumVersion")
        .filter_map(|layer| valid_version(layer.settings.get("minimumVersion")))
        .reduce(higher);
    VersionPolicy {
        channel: channel.unwrap_or(Channel::Latest),
        minimum_version: minimum,
        required_minimum_version: managed_version(&ordered, "requiredMinimumVersion"),
        required_maximum_version: managed_version(&ordered, "requiredMaximumVersion"),
    }
}

/// What the host knows of the releases: the installed version and what each channel serves.
#[derive(Clone, Debug, Deserialize)]
#[expect(missing_docs, reason = "each field is the version of its name")]
pub struct KnownVersions {
    pub installed: String,
    pub latest: String,
    pub stable: String,
}

/// Why an update installs nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(missing_docs, reason = "each variant is the reason of its name")]
pub enum SkipReason {
    ChannelVersionUnknown,
    AlreadyCurrent,
    AboveRequiredMaximum,
    BelowRequiredMinimum,
    BelowMinimumVersion,
}

/// What an update installs, or why it skips.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum UpdatePick {
    /// The channel's version.
    Install {
        /// The version.
        version: String,
        /// Its channel.
        from: Channel,
        /// The channel's version is older than the installed one (moving to `stable` from a newer `latest`).
        downgrade: bool,
    },
    /// Nothing.
    Skip {
        /// Why.
        reason: SkipReason,
    },
}

struct Limit {
    reason: SkipReason,
    bound: fn(&VersionPolicy) -> Option<&str>,
    breaks: fn(Ordering) -> bool,
}

/// The limits an update respects, in the order they are reported.
const LIMITS: [Limit; 3] = [
    Limit {
        reason: SkipReason::AboveRequiredMaximum,
        bound: |policy| policy.required_maximum_version.as_deref(),
        breaks: Ordering::is_gt,
    },
    // Judgment: a version below the floor would not start, so an update skips it too.
    Limit {
        reason: SkipReason::BelowRequiredMinimum,
        bound: |policy| policy.required_minimum_version.as_deref(),
        breaks: Ordering::is_lt,
    },
    Limit {
        reason: SkipReason::BelowMinimumVersion,
        bound: |policy| policy.minimum_version.as_deref(),
        breaks: Ordering::is_lt,
    },
];

fn violated(policy: &VersionPolicy, candidate: &str) -> Option<SkipReason> {
    LIMITS
        .iter()
        .find(|limit| {
            let order = (limit.bound)(policy).and_then(|bound| compare_versions(candidate, bound));
            order.is_some_and(limit.breaks)
        })
        .map(|limit| limit.reason)
}

fn served_by(channel: Channel, versions: &KnownVersions) -> &str {
    match channel {
        Channel::Latest => &versions.latest,
        Channel::Stable => &versions.stable,
    }
}

/// What a background auto-update or `claude update` installs: the channel's version, unless it is
/// already installed or the policy rules it out. Nothing else is offered, so a refused version
/// leaves the installation where it is ("skip versions above the ceiling, so an installation inside
/// the range stays inside it").
#[must_use]
pub fn pick_allowed_version(policy: &VersionPolicy, versions: &KnownVersions) -> UpdatePick {
    let candidate = normalized_version(served_by(policy.channel, versions));
    let installed = normalized_version(&versions.installed);
    let Some(candidate) = candidate else {
        return UpdatePick::Skip {
            reason: SkipReason::ChannelVersionUnknown,
        };
    };
    let order = installed.map_or(Ordering::Greater, |installed| {
        compare_versions(&candidate, &installed).unwrap_or(Ordering::Greater)
    });
    if order == Ordering::Equal {
        return UpdatePick::Skip {
            reason: SkipReason::AlreadyCurrent,
        };
    }
    if let Some(reason) = violated(policy, &candidate) {
        return UpdatePick::Skip { reason };
    }
    UpdatePick::Install {
        version: candidate,
        from: policy.channel,
        downgrade: order.is_lt(),
    }
}

/// The commands the required range does and does not stop: everything else is a start.
pub const CLAUDE_COMMANDS: [&str; 4] = ["start", "update", "install", "doctor"];

/// "`claude update`, `claude install`, and `claude doctor` keep working" outside the range "so
/// users can recover".
const KEEPS_WORKING: [(&str, bool); 4] = [
    ("start", false),
    ("update", true),
    ("install", true),
    ("doctor", true),
];

/// Whether Claude Code starts.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StartVerdict {
    /// It starts.
    pub allowed: bool,
    /// Why not: `below-required-minimum` or `above-required-maximum`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<SkipReason>,
    /// What the user is told.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<&'static str>,
}

/// Wording is a judgment (spec/judgments.md): the docs say what the user is told, not the sentence.
const BELOW: &str = "This Claude Code version is older than your organization allows. Update through your organization's approved method.";
const ABOVE: &str = "This Claude Code version is newer than your organization allows. Install an approved version through your organization's approved method; `claude install <version>` may also work.";

/// A start to check.
#[derive(Clone, Debug, Deserialize)]
pub struct StartRequest {
    /// The command.
    pub command: String,
    /// The running version.
    pub current: String,
}

const ALLOWED: StartVerdict = StartVerdict {
    allowed: true,
    reason: None,
    message: None,
};

/// "When the running version is older \[newer\], Claude Code exits at startup". The check is made
/// once, when Claude Code starts: a session already running continues, so a host calls this at
/// launch only.
#[must_use]
pub fn start_gate(policy: &VersionPolicy, request: &StartRequest) -> StartVerdict {
    if KEEPS_WORKING
        .iter()
        .any(|(command, keeps)| *command == request.command && *keeps)
    {
        return ALLOWED;
    }
    let order = |bound: &str| compare_versions(&request.current, bound).unwrap_or(Ordering::Equal);
    if policy
        .required_minimum_version
        .as_deref()
        .is_some_and(|floor| order(floor).is_lt())
    {
        let reason = Some(SkipReason::BelowRequiredMinimum);
        return StartVerdict {
            allowed: false,
            reason,
            message: Some(BELOW),
        };
    }
    if policy
        .required_maximum_version
        .as_deref()
        .is_some_and(|ceiling| order(ceiling).is_gt())
    {
        let reason = Some(SkipReason::AboveRequiredMaximum);
        return StartVerdict {
            allowed: false,
            reason,
            message: Some(ABOVE),
        };
    }
    ALLOWED
}

/// The `/config` choice when moving from `latest` to `stable`: allow the downgrade, or stay on the
/// current version.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "each variant is the answer of its name")]
pub enum DowngradeAnswer {
    Downgrade,
    Stay,
}

/// A channel picked in `/config`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSwitch {
    /// The channel followed now; picking it again changes nothing.
    pub from: Channel,
    /// The channel picked.
    pub to: Channel,
    /// The running version.
    pub current_version: String,
    /// Needed to leave `latest` for `stable`; without it the switch asks.
    #[serde(default)]
    pub answer: Option<DowngradeAnswer>,
}

/// What the user settings change.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SwitchPlan {
    /// The switch asks first.
    Ask {
        /// Always `downgrade-or-stay`.
        ask: &'static str,
    },
    /// What is written and removed.
    Change {
        /// What is written.
        set: SwitchSet,
        /// What is removed.
        remove: Vec<&'static str>,
    },
}

/// The keys a switch writes.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSet {
    /// Always `stable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_updates_channel: Option<Channel>,
    /// The version stayed on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<String>,
}

fn change(set: SwitchSet, remove: Vec<&'static str>) -> SwitchPlan {
    SwitchPlan::Change { set, remove }
}

/// Judgment (spec/judgments.md): choosing the channel already followed writes nothing and asks nothing.
fn no_change() -> SwitchPlan {
    change(SwitchSet::default(), vec![])
}

fn to_stable(answer: DowngradeAnswer, current: &str) -> SwitchPlan {
    let stable = Some(Channel::Stable);
    match answer {
        DowngradeAnswer::Downgrade => change(
            SwitchSet {
                auto_updates_channel: stable,
                minimum_version: None,
            },
            vec![],
        ),
        DowngradeAnswer::Stay => change(
            SwitchSet {
                auto_updates_channel: stable,
                minimum_version: Some(current.to_owned()),
            },
            vec![],
        ),
    }
}

fn switch(request: &ChannelSwitch) -> SwitchPlan {
    match request.to {
        Channel::Stable => match request.answer {
            None => SwitchPlan::Ask {
                ask: "downgrade-or-stay",
            },
            Some(answer) => to_stable(answer, &request.current_version),
        },
        Channel::Latest => change(
            SwitchSet::default(),
            vec!["autoUpdatesChannel", "minimumVersion"],
        ),
    }
}

/// What the user settings change when the channel is picked under **Auto-update channel** in
/// `/config`: "stable" is written (asking first about the downgrade, and staying sets
/// `minimumVersion`); switching back to latest removes the key and clears `minimumVersion`. Only
/// "from `latest` to `stable`" asks.
#[must_use]
pub fn channel_switch(request: &ChannelSwitch) -> SwitchPlan {
    if request.from == request.to {
        no_change()
    } else {
        switch(request)
    }
}

crate::registry::exports! {
    "updates/claude-code-version-policy" {
        "CHANNELS" => ok || CHANNELS;
        "readVersionPolicy" => ok |layers: Vec<PolicyLayer>| read_version_policy(&layers);
        "pickAllowedVersion" => ok |policy: VersionPolicy, versions: KnownVersions|
            pick_allowed_version(&policy, &versions);
        "CLAUDE_COMMANDS" => ok || CLAUDE_COMMANDS;
        "startGate" => ok |policy: VersionPolicy, request: StartRequest| start_gate(&policy, &request);
        "channelSwitch" => ok |request: ChannelSwitch| channel_switch(&request);
    }
}
