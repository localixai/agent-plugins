//! Port of `src/settings/managed/admin-source.ts`, switched.
//!
//! The managed sources and what a machine can hold in each (managed-settings.md, "Choose a delivery
//! mechanism", "How Claude Code combines managed sources", "When an admin document counts as
//! present", "Find entries Claude Code dropped"). The kit reads no file, plist or registry: the host
//! says what it found at each source (`SourceRead`) and this module decides what that document is.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error, warning};
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue};
use crate::settings::broken_file::broken_source;
use crate::settings::layers::PolicyScope;
use crate::settings::read_layer::read_settings_layer;

/// A source of the managed composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagedSource {
    /// Server-managed settings.
    Remote,
    /// A macOS configuration profile.
    Plist,
    /// The `HKLM` registry.
    Hklm,
    /// The managed file and its drop-ins.
    File,
    /// The `HKCU` registry.
    Hkcu,
}

impl ManagedSource {
    /// The source's name as the package writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Remote => "remote",
            Self::Plist => "plist",
            Self::Hklm => "hklm",
            Self::File => "file",
            Self::Hkcu => "hkcu",
        }
    }
}

/// The managed sources.
pub const MANAGED_SOURCES: [ManagedSource; 5] = [
    ManagedSource::Remote,
    ManagedSource::Plist,
    ManagedSource::Hklm,
    ManagedSource::File,
    ManagedSource::Hkcu,
];

/// Why a document exists and could not be read: "the operating system denied the read" or any other failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnreadableCause {
    /// The operating system denied the read.
    Denied,
    /// Any other failure.
    Failed,
}

/// The causes.
pub const UNREADABLE_CAUSES: [UnreadableCause; 2] =
    [UnreadableCause::Denied, UnreadableCause::Failed];

/// A source that exists and could not be read: the cause and the underlying error's text (errors.md, `Detail: <source>: <reason>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnreadableRead {
    /// The cause.
    pub unreadable: UnreadableCause,
    /// The error's text.
    pub reason: String,
}

/// What the host found at one source. `undefined` (`None` where it is read): nothing there (not a
/// failure). `value`: the content, either the parsed JSON or its text (a plist the host converted
/// with `plutil` comes as the converted JSON). `unreadable`: it exists and the read failed, with the
/// error. `malformed`: the platform converter (`plutil`) reported the document malformed, with its
/// message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceRead {
    /// It exists and the read failed.
    Unreadable(UnreadableRead),
    /// The converter reported it malformed.
    Malformed {
        /// The converter's message.
        malformed: String,
    },
    /// The content.
    Value {
        /// The parsed JSON or its text.
        #[serde(default)]
        value: JsValue,
    },
}

/// The control keys: neither counts as a policy key (managed-settings.md, "How Claude Code combines managed sources").
pub const CONTROL_KEYS: [&str; 2] = ["managedSourcesBehavior", "wslInheritsWindowsSettings"];

/// Where a document's text becomes an object: each stored form fails in its own ways.
type Parsed = Result<JsObject, String>;

const NOT_OBJECT: &str = "its top level is not a JSON object";

fn parse_text(text: &str) -> Parsed {
    match json_parse(text) {
        Ok(JsValue::Object(object)) => Ok(object),
        Ok(_) => Err(NOT_OBJECT.to_owned()),
        Err(failure) => Err(failure),
    }
}

fn as_object(value: &JsValue) -> Parsed {
    match value {
        JsValue::Object(object) => Ok(object.clone()),
        _ => Err(NOT_OBJECT.to_owned()),
    }
}

/// The forms a stored policy takes, each with what makes it unparseable (managed-settings.md, "Find entries Claude Code dropped").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Form {
    /// The payload of a fetch, already JSON.
    Payload,
    /// A file or drop-in: "the file isn't valid JSON, or its top level isn't an object"; an empty one counts as `{}`.
    JsonFile,
    /// An MDM plist: the content `plutil` converted must be a JSON object.
    Plist,
    /// A registry value: "the `Settings` value isn't a string, is empty, or doesn't hold a JSON object".
    RegistryValue,
}

impl Form {
    fn parse(self, value: &JsValue) -> Parsed {
        let text = value.as_str();
        match self {
            Self::Payload => as_object(value),
            Self::JsonFile => match text {
                None => as_object(value),
                Some(text) if crate::js::trim(text).is_empty() => Ok(JsObject::new()),
                Some(text) => parse_text(text),
            },
            Self::Plist => text.map_or_else(|| as_object(value), parse_text),
            Self::RegistryValue => match text {
                None => Err("the Settings value is not a string".to_owned()),
                Some("") => Err("the Settings value is empty".to_owned()),
                Some(text) => parse_text(text),
            },
        }
    }
}

/// What a source is.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTraits {
    /// The policy is stored on the machine ("endpoint-managed"), not fetched.
    pub on_machine: bool,
    /// A higher rank is read first. Equal ranks are alternatives for one OS (a plist or the HKLM key), never both.
    pub rank: u8,
    /// "Admin source: one of the first three sources below. The HKCU registry is user-writable and isn't one."
    pub admin: bool,
    /// The form it is stored in.
    pub form: Form,
    /// How `/status` names it in `Setting sources`.
    pub status_label: &'static str,
}

/// Each source's traits.
#[must_use]
pub const fn source_traits(source: ManagedSource) -> SourceTraits {
    let (on_machine, rank, admin, form, status_label) = match source {
        ManagedSource::Remote => (false, 4, true, Form::Payload, "remote"),
        ManagedSource::Plist => (true, 3, true, Form::Plist, "plist"),
        ManagedSource::Hklm => (true, 3, true, Form::RegistryValue, "HKLM"),
        ManagedSource::File => (true, 2, true, Form::JsonFile, "file"),
        ManagedSource::Hkcu => (true, 1, false, Form::RegistryValue, "HKCU"),
    };
    SourceTraits {
        on_machine,
        rank,
        admin,
        form,
        status_label,
    }
}

/// Where and why a source could not be read: the label a report names the source by, and the error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnreadableAt {
    /// The label.
    pub label: String,
    /// The error.
    pub reason: String,
}

/// One source after reading: what it delivers and whether it counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDocument {
    /// The source.
    pub source: ManagedSource,
    /// What the file says, repaired as a managed document is (`readSettingsLayer`); `{}` when absent, unreadable or broken.
    pub settings: JsValue,
    /// The parsed object before repair, for the control keys and the presence rule.
    pub raw: JsObject,
    /// The keys the document carries with a value no repair could keep (`PolicyLayer.invalid`).
    pub invalid: Vec<String>,
    /// "A document is present when it sets any policy key to a value other than null", or exists and cannot be read.
    pub present: bool,
    /// It carries at least one policy key, so it can be the policy.
    pub delivers: bool,
    /// Present and not a JSON object: an admin one stops Claude Code from starting.
    pub broken: bool,
    /// What reading it reported.
    pub issues: Vec<PluginIssue>,
    /// Why it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<UnreadableCause>,
    /// Where and why it could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreadable_at: Option<UnreadableAt>,
    /// How the file source is made of files: `file`, `drop-ins` or `file + drop-ins`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
}

/// Any key other than the two control keys, set to a value other than `null`.
fn delivers_policy_key(raw: &JsObject) -> bool {
    raw.iter()
        .any(|(name, value)| !value.is_null() && !CONTROL_KEYS.contains(&name.as_str()))
}

/// The document of a source that has nothing.
#[must_use]
pub fn absent(source: ManagedSource) -> AdminDocument {
    AdminDocument {
        source,
        settings: JsValue::Object(JsObject::new()),
        raw: JsObject::new(),
        invalid: Vec::new(),
        present: false,
        delivers: false,
        broken: false,
        issues: Vec::new(),
        unreadable: None,
        unreadable_at: None,
        shape: None,
    }
}

/// The failing document: an admin one is fatal (it names the source), the user-writable one is a notice.
fn broken_document(source: ManagedSource, why: &str, label: Option<&str>) -> AdminDocument {
    let named = label.map_or_else(String::new, |label| format!("{label}: "));
    let admin = source_traits(source).admin;
    let issue = if admin {
        broken_source(PolicyScope::Managed, Some(why), label)
    } else {
        warning(
            "managed:HKCU",
            &format!("{named}the HKCU registry value is not a JSON object and is ignored ({why})"),
        )
    };
    AdminDocument {
        broken: admin,
        present: admin,
        issues: vec![issue],
        ..absent(source)
    }
}

/// What one source's read comes to. `label` names the source in a report (a path, `per-user managed
/// preferences`, `Registry: HKLM\SOFTWARE\Policies\ClaudeCode\Settings`).
#[must_use]
pub fn read_admin_document(
    source: ManagedSource,
    read: Option<&SourceRead>,
    label: Option<&str>,
) -> AdminDocument {
    let traits = source_traits(source);
    let value = match read {
        None => return absent(source),
        // The user-writable registry never blocks launch and is never an admin document.
        Some(SourceRead::Unreadable(read)) if traits.admin => {
            return AdminDocument {
                present: true,
                unreadable: Some(read.unreadable),
                unreadable_at: Some(UnreadableAt {
                    label: label.unwrap_or(traits.status_label).to_owned(),
                    reason: read.reason.clone(),
                }),
                ..absent(source)
            };
        }
        Some(SourceRead::Unreadable(_)) => return absent(source),
        Some(SourceRead::Malformed { malformed }) => {
            return broken_document(source, malformed, label);
        }
        Some(SourceRead::Value { value }) => value,
    };
    let object = match traits.form.parse(value) {
        Ok(object) => object,
        Err(why) => return broken_document(source, &why, label),
    };
    let layer = read_settings_layer(
        PolicyScope::Managed,
        &JsValue::Object(object.clone()),
        label,
    );
    let delivers = delivers_policy_key(&object);
    AdminDocument {
        settings: layer.layer.settings,
        raw: object,
        invalid: layer.layer.invalid.unwrap_or_default(),
        present: delivers,
        delivers,
        issues: layer.issues,
        ..absent(source)
    }
}

/// What a failure to read a source costs, when no admin source supplies a policy (managed-settings.md, errors.md).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnreadableVerdict {
    /// Claude Code exits at startup.
    pub exits: bool,
    /// `/status` and `claude doctor` record it, and a `-p` run prints it to stderr.
    pub recorded: bool,
    /// The message.
    pub message: String,
}

const UNREADABLE_EXIT: [&str; 3] = [
    "Unable to read managed policy settings.",
    "This machine may require organization login enforcement, but the policy file failed to load.",
    "Contact your administrator.",
];

/// What each cause comes to: the operating system's denial starts the session without the source; any other failure exits.
fn unreadable_verdict_of(cause: UnreadableCause, detail: &str) -> UnreadableVerdict {
    match cause {
        UnreadableCause::Denied => UnreadableVerdict {
            exits: false,
            recorded: true,
            message: format!(
                "{detail}: the operating system denied the read, so this session starts without that source's policies"
            ),
        },
        UnreadableCause::Failed => UnreadableVerdict {
            exits: true,
            recorded: true,
            message: format!("{}\n\nDetail: {detail}", UNREADABLE_EXIT.join("\n")),
        },
    }
}

/// A cause that exits outranks one that starts the session without the source: the stricter outcome wins.
const fn cause_rank(cause: UnreadableCause) -> u8 {
    match cause {
        UnreadableCause::Denied => 0,
        UnreadableCause::Failed => 1,
    }
}

/// The stricter of two causes, `a` on a tie.
#[must_use]
pub const fn harsher_cause(a: UnreadableCause, b: UnreadableCause) -> UnreadableCause {
    if cause_rank(b) > cause_rank(a) { b } else { a }
}

/// An issue about a failed read, an error where the cause exits and a warning where it does not.
#[must_use]
pub fn unreadable_issue(cause: UnreadableCause, path: &str, message: &str) -> PluginIssue {
    match cause {
        UnreadableCause::Denied => warning(path, message),
        UnreadableCause::Failed => error(path, message),
    }
}

/// The verdict on a source that could not be read.
#[must_use]
pub fn unreadable_verdict(cause: UnreadableCause, source: &str, reason: &str) -> UnreadableVerdict {
    unreadable_verdict_of(cause, &format!("{source}: {reason}"))
}

/// The issue of a verdict, an error when it stops Claude Code.
#[must_use]
pub fn verdict_issue(verdict: &UnreadableVerdict, path: &str) -> PluginIssue {
    if verdict.exits {
        error(path, &verdict.message)
    } else {
        warning(path, &verdict.message)
    }
}

crate::registry::exports! {
    "settings/managed/admin-source" {
        "MANAGED_SOURCES" => ok || MANAGED_SOURCES;
        "UNREADABLE_CAUSES" => ok || UNREADABLE_CAUSES;
        "CONTROL_KEYS" => ok || CONTROL_KEYS;
        "SOURCE_TRAITS" => ok || MANAGED_SOURCES
            .into_iter()
            .map(|source| (source, source_traits(source)))
            .collect::<IndexMap<ManagedSource, SourceTraits>>();
        "readAdminDocument" => ok |source: ManagedSource, read: Option<SourceRead>, label: Option<String>|
            read_admin_document(source, read.as_ref(), label.as_deref());
        "harsherCause" => ok |a: UnreadableCause, b: UnreadableCause| harsher_cause(a, b);
        "unreadableIssue" => ok |cause: UnreadableCause, path: String, message: String| unreadable_issue(cause, &path, &message);
        "unreadableVerdict" => ok |cause: UnreadableCause, source: String, reason: String| unreadable_verdict(cause, &source, &reason);
        "verdictIssue" => ok |verdict: UnreadableVerdict, path: String| verdict_issue(&verdict, &path);
    }
}
