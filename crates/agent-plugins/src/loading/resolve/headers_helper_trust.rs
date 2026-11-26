//! Port of `src/loading/resolve/headers-helper-trust.ts`, switched.

use serde::{Deserialize, Serialize};

use super::headers_helper_origins::{HelperContext, TrustGate};
use crate::format::config_directory::{CONFIG_DIR_VARIABLE, JoinPath, global_config_file_of};
use crate::js::JsValue;
use crate::loading::trust_gates::{RepositoryContent, TrustSituation, content_gate};
use crate::ports::locations::Locations;
use crate::protocol::is_record::is_record;
use crate::registry::Port;

/// How a folder came to be trusted (mcp.md, "Trust a folder before its headersHelper runs"). Only the
/// folder's own persisted trust counts: "a parent folder's trust, and the automatic trust a `claude -p`
/// or SDK session gets for hooks in settings files" do not. The host says which it has; the kit reads
/// no dialog and no `~/.claude.json` but through `persistedTrust`.
pub const TRUST_SOURCES: [TrustSource; 4] = [
    TrustSource::Persisted,
    TrustSource::ParentFolder,
    TrustSource::AutomaticSession,
    TrustSource::None,
];

/// One of `TRUST_SOURCES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustSource {
    /// The folder's own persisted trust.
    Persisted,
    /// A parent folder's trust.
    ParentFolder,
    /// The automatic trust of a `-p` or SDK session.
    AutomaticSession,
    /// No trust.
    None,
}

/// The column of the trust table (`trust-gates`) each source is: only persisted trust is the folder's
/// own; the automatic trust of a `-p` or SDK session is the `claude -p` column.
const fn situation(source: TrustSource) -> TrustSituation {
    match source {
        TrustSource::Persisted => TrustSituation::Trusted,
        TrustSource::ParentFolder => TrustSituation::ParentTrusted,
        TrustSource::AutomaticSession => TrustSituation::NonInteractive,
        TrustSource::None => TrustSituation::Dialog,
    }
}

const fn counts(content: RepositoryContent, trust: &HelperTrust) -> bool {
    content_gate(content, situation(trust.source)).applies
}

/// What the gate reads of the folder's trust.
#[derive(Clone, Debug, Deserialize)]
pub struct HelperTrust {
    /// How the folder the server is declared in came to be trusted.
    pub source: TrustSource,
    /// The folder trust is keyed on (permissions.md, "Project allow rules and workspace trust").
    pub key: String,
    /// An interactive session; the refusal goes to the debug log there and to stderr otherwise.
    pub interactive: bool,
}

/// "Set `projects["<path>"].hasTrustDialogAccepted` to `true` in `~/.claude.json`": that folder's
/// persisted trust, read from the parsed file. Another path's entry, a parent's, a non-`true` value
/// and a file of another shape are no trust.
#[must_use]
pub fn persisted_trust(claude_json: &JsValue, key: &str) -> TrustSource {
    let projects = claude_json
        .get("projects")
        .filter(|_| is_record(claude_json));
    let entry = projects
        .filter(|projects| is_record(projects))
        .and_then(|projects| projects.get(key));
    let accepted = entry
        .filter(|entry| is_record(entry))
        .and_then(|entry| entry.get("hasTrustDialogAccepted"))
        .and_then(JsValue::as_bool);
    if accepted == Some(true) {
        TrustSource::Persisted
    } else {
        TrustSource::None
    }
}

/// Where a refusal's line goes.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum NoticeChannel {
    /// "Claude Code writes this line in non-interactive mode only".
    #[serde(rename = "stderr")]
    Stderr,
    /// An interactive session logs it.
    #[serde(rename = "debug-log")]
    DebugLog,
}

/// The line the host shows and where.
#[derive(Clone, Debug, Serialize)]
pub struct HelperNotice {
    /// The line.
    pub text: String,
    /// "Claude Code writes this line in non-interactive mode only"; an interactive session logs it.
    pub channel: NoticeChannel,
}

/// The line errors.md quotes under "headersHelper not run".
async fn refusal(name: &str, trust: &HelperTrust, context: &HelperContext<Port>) -> HelperNotice {
    let locations = &context.locations;
    let file = global_config_file_of(
        &locations.home_dir().await,
        context
            .environment
            .get(CONFIG_DIR_VARIABLE)
            .map(String::as_str),
        &JoinPath(locations),
    )
    .await;
    HelperNotice {
        text: format!(
            "MCP server '{name}': headersHelper not run \u{2014} this workspace has no persisted trust; accept the trust dialog here once interactively, or set projects[\"{}\"].hasTrustDialogAccepted in {file}.",
            trust.key
        ),
        channel: if trust.interactive {
            NoticeChannel::DebugLog
        } else {
            NoticeChannel::Stderr
        },
    }
}

/// Whether the gate lets the helper through; otherwise what the host does instead.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum GateResult {
    /// The helper may run.
    Open {
        /// Always true.
        open: bool,
    },
    /// The helper does not run; the server connects with its static headers alone, with a notice.
    Refused {
        /// Always false.
        open: bool,
        /// Always true.
        loaded: bool,
        /// What the host says.
        notice: HelperNotice,
    },
    /// The server does not load at all.
    NotLoaded {
        /// Always false.
        open: bool,
        /// Always false.
        loaded: bool,
    },
}

const OPEN: GateResult = GateResult::Open { open: true };

/// `helper`: a project or local server connects with its static `headers` alone, with the notice.
/// `load`: an agent file's inline server from the project or an `--add-dir` directory "doesn't load at
/// all" until that folder is trusted.
pub async fn pass_gate(
    gate: TrustGate,
    name: &str,
    trust: &HelperTrust,
    context: &HelperContext<Port>,
) -> GateResult {
    match gate {
        TrustGate::None => OPEN,
        TrustGate::Helper => {
            if counts(RepositoryContent::HeadersHelper, trust) {
                OPEN
            } else {
                GateResult::Refused {
                    open: false,
                    loaded: true,
                    notice: refusal(name, trust, context).await,
                }
            }
        }
        TrustGate::Load => {
            if counts(RepositoryContent::SubagentInlineMcp, trust) {
                OPEN
            } else {
                GateResult::NotLoaded {
                    open: false,
                    loaded: false,
                }
            }
        }
    }
}

crate::registry::exports! {
    "loading/resolve/headers-helper-trust" {
        "TRUST_SOURCES" => ok || TRUST_SOURCES;
        "persistedTrust" => ok |claude_json: JsValue, key: String| persisted_trust(&claude_json, &key);
        "passGate" => async ok |gate: TrustGate, name: String, trust: HelperTrust, context: HelperContext<Port>|
            pass_gate(gate, &name, &trust, &context).await;
    }
}
