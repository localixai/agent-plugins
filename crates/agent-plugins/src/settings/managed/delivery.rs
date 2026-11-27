//! Port of `src/settings/managed/delivery.ts`, switched.
//!
//! The delivery mechanisms as data (managed-settings.md, "Deploy a managed settings file", "Choose a
//! delivery mechanism", "Where each mechanism stores the policy"): where each stores the policy, how
//! often Claude Code reads it, and which source of the composition it is.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::JsValue;
use crate::settings::managed::admin_source::ManagedSource;

/// The platforms.
pub const PLATFORMS: [Platform; 4] = [
    Platform::Macos,
    Platform::Linux,
    Platform::Wsl,
    Platform::Windows,
];

/// One of `PLATFORMS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// macOS.
    Macos,
    /// Linux.
    Linux,
    /// Windows Subsystem for Linux.
    Wsl,
    /// Windows.
    Windows,
}

/// The mechanisms.
pub const MECHANISMS: [Mechanism; 4] = [
    Mechanism::ServerManaged,
    Mechanism::Mdm,
    Mechanism::File,
    Mechanism::Hkcu,
];

/// One of `MECHANISMS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mechanism {
    /// Server-managed settings.
    ServerManaged,
    /// A configuration profile or the `HKLM` registry.
    Mdm,
    /// The managed file.
    File,
    /// The `HKCU` registry.
    Hkcu,
}

/// What one mechanism does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanismTraits {
    /// The sources of the composition this mechanism fills, on a machine that has it.
    pub sources: &'static [ManagedSource],
    /// "Read at startup" holds for every mechanism.
    pub read_at_startup: bool,
    /// Minutes between checks for a change; absent when the mechanism is not polled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_every_minutes: Option<u32>,
    /// "Reloaded when a file changes".
    pub reloads_on_change: bool,
    /// "The MDM and file rows are together called endpoint-managed settings, because the policy is stored on the developer's device."
    pub endpoint_managed: bool,
}

/// The traits of each mechanism, in `MECHANISMS`' order.
pub const MECHANISM_TRAITS: [(Mechanism, MechanismTraits); 4] = [
    (
        Mechanism::ServerManaged,
        MechanismTraits {
            sources: &[ManagedSource::Remote],
            read_at_startup: true,
            poll_every_minutes: Some(60),
            reloads_on_change: false,
            endpoint_managed: false,
        },
    ),
    (
        Mechanism::Mdm,
        MechanismTraits {
            sources: &[ManagedSource::Plist, ManagedSource::Hklm],
            read_at_startup: true,
            poll_every_minutes: Some(30),
            reloads_on_change: false,
            endpoint_managed: true,
        },
    ),
    (
        Mechanism::File,
        MechanismTraits {
            sources: &[ManagedSource::File],
            read_at_startup: true,
            poll_every_minutes: None,
            reloads_on_change: true,
            endpoint_managed: true,
        },
    ),
    (
        Mechanism::Hkcu,
        MechanismTraits {
            sources: &[ManagedSource::Hkcu],
            read_at_startup: true,
            poll_every_minutes: Some(30),
            reloads_on_change: false,
            endpoint_managed: false,
        },
    ),
];

/// The traits of one mechanism.
fn traits_of(mechanism: Mechanism) -> MechanismTraits {
    MECHANISM_TRAITS
        .iter()
        .find(|(one, _)| *one == mechanism)
        .map_or(MECHANISM_TRAITS[0].1, |(_, traits)| *traits)
}

/// When Claude Code last read a mechanism.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RereadFacts {
    /// Minutes since the read.
    pub minutes_since_read: f64,
    /// The file changed on disk since it was read.
    pub changed: bool,
}

/// Whether Claude Code reads the mechanism again now: on its poll interval, or when its file changes.
#[must_use]
pub fn reread_due(mechanism: Mechanism, facts: &RereadFacts) -> bool {
    let traits = traits_of(mechanism);
    let polled = traits
        .poll_every_minutes
        .is_some_and(|every| facts.minutes_since_read >= f64::from(every));
    polled || (traits.reloads_on_change && facts.changed)
}

/// The MDM mechanism a platform has: a configuration profile on macOS, the `HKLM` registry on Windows; Linux has neither.
#[must_use]
pub const fn mdm_source(platform: Platform) -> Option<ManagedSource> {
    match platform {
        Platform::Macos => Some(ManagedSource::Plist),
        Platform::Linux => None,
        Platform::Wsl | Platform::Windows => Some(ManagedSource::Hklm),
    }
}

/// "The `com.anthropic.claudecode` managed preferences domain."
pub const PLIST_DOMAIN: &str = "com.anthropic.claudecode";

/// The registry: "the JSON as a `REG_SZ` or `REG_EXPAND_SZ` value named `Settings` under `SOFTWARE\Policies\ClaudeCode`".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RegistryValue {
    /// The value's name.
    pub name: &'static str,
    /// The types it may have.
    pub types: [&'static str; 2],
    /// The key it sits under.
    pub key: &'static str,
}

/// See [`RegistryValue`].
pub const REGISTRY_VALUE: RegistryValue = RegistryValue {
    name: "Settings",
    types: ["REG_SZ", "REG_EXPAND_SZ"],
    key: "SOFTWARE\\Policies\\ClaudeCode",
};

/// The registry hives.
pub const HIVES: [Hive; 2] = [Hive::Hklm, Hive::Hkcu];

/// One of `HIVES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hive {
    /// `HKLM`.
    #[serde(rename = "HKLM")]
    Hklm,
    /// `HKCU`.
    #[serde(rename = "HKCU")]
    Hkcu,
}

impl Hive {
    /// The hive's name as the registry spells it.
    const fn name(self) -> &'static str {
        match self {
            Self::Hklm => "HKLM",
            Self::Hkcu => "HKCU",
        }
    }
}

/// The registry path of a hive's policy value, as `/status` and errors write it.
#[must_use]
pub fn registry_path(hive: Hive) -> String {
    format!(
        "{}\\{}\\{}",
        hive.name(),
        REGISTRY_VALUE.key,
        REGISTRY_VALUE.name
    )
}

/// The system directory of each platform, and the separator its paths use.
const fn system_directory(platform: Platform) -> (&'static str, &'static str) {
    match platform {
        Platform::Macos => ("/Library/Application Support/ClaudeCode", "/"),
        Platform::Linux | Platform::Wsl => ("/etc/claude-code", "/"),
        Platform::Windows => ("C:\\Program Files\\ClaudeCode", "\\"),
    }
}

/// Where the file-based mechanism keeps its files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManagedFilePaths {
    /// The directory.
    pub dir: String,
    /// `managed-settings.json`.
    pub settings: String,
    /// `managed-settings.d/`, the drop-in directory.
    #[serde(rename = "dropIns")]
    pub drop_ins: String,
    /// `managed-mcp.json`.
    pub mcp: String,
}

/// Where the file-based mechanism keeps its files.
#[must_use]
pub fn managed_file_paths(platform: Platform) -> ManagedFilePaths {
    let (dir, separator) = system_directory(platform);
    let inside = |name: &str| format!("{dir}{separator}{name}");
    ManagedFilePaths {
        dir: dir.to_owned(),
        settings: inside("managed-settings.json"),
        drop_ins: inside("managed-settings.d"),
        mcp: inside("managed-mcp.json"),
    }
}

/// Whether Claude Code reads `path` as the managed settings file of `platform`.
#[must_use]
pub fn reads_managed_settings_at(platform: Platform, path: &str) -> bool {
    path == managed_file_paths(platform).settings
}

/// A managed file deployed as a symbolic link.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymlinkFacts {
    /// The deployed file's path.
    pub path: String,
    /// Whether the link's target is inside the system directory; `undefined` when Claude Code cannot confirm it.
    #[serde(default)]
    pub target_inside: Option<bool>,
}

/// What the link means for the policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SymlinkVerdict {
    /// "The policy still applies" whatever the link leads to.
    pub applies: bool,
    /// The warning `/status` and `claude doctor` show, naming the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<PluginIssue>,
}

/// A managed file deployed as a symbolic link: "Claude Code reads the file the link leads to. The
/// policy still applies when the link leads outside the system directory or Claude Code can't confirm
/// that it stays inside. A warning naming the file then appears in `/status` and `claude doctor`."
#[must_use]
pub fn symlink_verdict(facts: &SymlinkFacts) -> SymlinkVerdict {
    if facts.target_inside == Some(true) {
        return SymlinkVerdict {
            applies: true,
            warning: None,
        };
    }
    SymlinkVerdict {
        applies: true,
        warning: Some(warning(
            &facts.path,
            &format!(
                "{} is a symbolic link that does not lead to a target inside the system directory; point the link at a target inside it to clear this warning",
                facts.path
            ),
        )),
    }
}

/// What one fetch of server-managed settings came to: the policy it delivered, or a failure.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ServerFetch {
    /// Whether the fetch succeeded.
    pub ok: bool,
    /// The policy a successful fetch delivered; `undefined` when it delivered none.
    #[serde(default)]
    pub settings: JsValue,
}

/// The local cache of server-managed settings: "Claude Code keeps a local cache that it applies at
/// startup and replaces on each successful fetch", so an edit to the cache "lasts only until the next
/// successful fetch" (managed-settings.md, "Where each mechanism stores the policy", "What a developer
/// can change"). A failed fetch leaves the cache as it is.
#[must_use]
pub fn cache_after_fetch(cached: &JsValue, fetch: &ServerFetch) -> JsValue {
    if fetch.ok {
        fetch.settings.clone()
    } else {
        cached.clone()
    }
}

/// How long a developer's edit of a managed source stays.
pub const EDIT_OUTCOMES: [EditOutcome; 3] = [
    EditOutcome::Stays,
    EditOutcome::UntilNextFetch,
    EditOutcome::Refused,
];

/// One of `EDIT_OUTCOMES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditOutcome {
    /// The edit stays.
    Stays,
    /// The edit lasts until the next successful fetch.
    UntilNextFetch,
    /// The edit is refused.
    Refused,
}

/// What a developer's own edit of the managed source does (managed-settings.md, "What a developer can
/// change"): "a developer who is an administrator on the machine can edit the managed source itself"
/// (the file, the MDM profile; MDM tooling can redeploy it); the HKCU registry is user-writable; an edit
/// to the server-managed cache lasts until the next successful fetch.
#[must_use]
pub const fn edit_of_managed_source(mechanism: Mechanism, local_admin: bool) -> EditOutcome {
    match mechanism {
        Mechanism::ServerManaged => EditOutcome::UntilNextFetch,
        Mechanism::Mdm | Mechanism::File => {
            if local_admin {
                EditOutcome::Stays
            } else {
                EditOutcome::Refused
            }
        }
        Mechanism::Hkcu => EditOutcome::Stays,
    }
}

crate::registry::exports! {
    "settings/managed/delivery" {
        "PLATFORMS" => ok || PLATFORMS;
        "MECHANISMS" => ok || MECHANISMS;
        "MECHANISM_TRAITS" => ok || MECHANISM_TRAITS.iter().copied().collect::<IndexMap<Mechanism, MechanismTraits>>();
        "rereadDue" => ok |mechanism: Mechanism, facts: RereadFacts| reread_due(mechanism, &facts);
        "MDM_SOURCE" => ok || PLATFORMS
            .into_iter()
            .map(|platform| (platform, mdm_source(platform).map_or(JsValue::Undefined, |source| JsValue::String(source.name().to_owned()))))
            .collect::<IndexMap<Platform, JsValue>>();
        "PLIST_DOMAIN" => ok || PLIST_DOMAIN;
        "REGISTRY_VALUE" => ok || REGISTRY_VALUE;
        "HIVES" => ok || HIVES;
        "registryPath" => ok |hive: Hive| registry_path(hive);
        "managedFilePaths" => ok |platform: Platform| managed_file_paths(platform);
        "readsManagedSettingsAt" => ok |platform: Platform, path: String| reads_managed_settings_at(platform, &path);
        "symlinkVerdict" => ok |facts: SymlinkFacts| symlink_verdict(&facts);
        "cacheAfterFetch" => ok |cached: JsValue, fetch: ServerFetch| cache_after_fetch(&cached, &fetch);
        "EDIT_OUTCOMES" => ok || EDIT_OUTCOMES;
        "editOfManagedSource" => ok |mechanism: Mechanism, local_admin: bool| edit_of_managed_source(mechanism, local_admin);
    }
}
