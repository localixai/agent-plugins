//! Port of `src/settings/mcp/managed-timing.ts`, switched.
//!
//! How `managedMcpServers` reaches the machine (managed-mcp.md, "When provided servers connect"):
//! through server-managed settings, which are fetched, through a Claude apps gateway sign-in, "Claude
//! Code loads the policy before the session starts", or from the device itself (MDM, registry,
//! `managed-settings.json`), judged to be read before the session too (spec/judgments.md).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::canonical_json::canonical_json;
use crate::js::JsValue;

/// The ways the managed servers reach the machine.
pub const MANAGED_DELIVERIES: [ManagedDelivery; 3] = [
    ManagedDelivery::ServerManaged,
    ManagedDelivery::Gateway,
    ManagedDelivery::Device,
];

/// One of `MANAGED_DELIVERIES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ManagedDelivery {
    /// Server-managed settings.
    ServerManaged,
    /// A Claude apps gateway sign-in.
    Gateway,
    /// The device itself.
    Device,
}

/// What a session knows when the servers connect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedTimingInput {
    /// How the servers reach the machine.
    pub delivery: ManagedDelivery,
    /// An interactive session; `false` for a `claude -p` run.
    pub interactive: bool,
    /// The machine has a cached copy of the server-managed settings.
    pub cached: bool,
    /// The server did not confirm the settings for this session.
    pub confirmation_failed: bool,
}

/// When the provided servers connect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "it mirrors the interface of the package, whose members are independent facts"
)]
pub struct ProvidedTiming {
    /// MCP servers wait for the server's confirmation before they load.
    pub waits_for_confirmation: bool,
    /// The provided servers are part of the session's first connection.
    pub connects_at_start: bool,
    /// An interactive first launch connects them as soon as the settings arrive.
    pub connects_on_arrival: bool,
    /// A `claude -p` run that has already started "can finish without them".
    pub may_finish_without: bool,
    /// What `/status` says when they are withheld.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withheld: Option<&'static str>,
}

const WITHHELD: &str = "Managed MCP servers are withheld: the server did not confirm the managed settings for this session";

const AT_START: ProvidedTiming = ProvidedTiming {
    waits_for_confirmation: false,
    connects_at_start: true,
    connects_on_arrival: false,
    may_finish_without: false,
    withheld: None,
};

/// "withholds the cached copy of this key until the server confirms", then "waits for that confirmation".
const fn cached_copy(input: ProvidedTimingInput) -> ProvidedTiming {
    ProvidedTiming {
        waits_for_confirmation: true,
        connects_at_start: !input.confirmation_failed,
        withheld: if input.confirmation_failed {
            Some(WITHHELD)
        } else {
            None
        },
        ..AT_START
    }
}

/// "On a machine's first launch, with nothing cached yet".
const fn first_launch(input: ProvidedTimingInput) -> ProvidedTiming {
    ProvidedTiming {
        connects_at_start: false,
        connects_on_arrival: input.interactive,
        may_finish_without: !input.interactive,
        ..AT_START
    }
}

/// When the provided servers connect in a session that starts (managed-mcp.md, "When provided servers connect").
#[must_use]
pub const fn provided_servers_timing(input: &ProvidedTimingInput) -> ProvidedTiming {
    match input.delivery {
        ManagedDelivery::ServerManaged => {
            if input.cached {
                cached_copy(*input)
            } else {
                first_launch(*input)
            }
        }
        ManagedDelivery::Gateway | ManagedDelivery::Device => AT_START,
    }
}

/// The servers a session's change touches, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ManagedServerChanges {
    /// Added.
    pub add: Vec<String>,
    /// Changed.
    pub change: Vec<String>,
    /// Removed.
    pub remove: Vec<String>,
}

/// A server some source defines (`McpCandidate` of `scope.ts`): what the comparison reads of it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct McpCandidate {
    /// The server's name.
    pub name: String,
    /// The server's entry.
    pub server: JsValue,
}

/// The session the change reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct ReconcileSession {
    /// An interactive session.
    pub interactive: bool,
}

/// The canonical text of each server, by name.
fn texts(candidates: &[McpCandidate]) -> IndexMap<&str, String> {
    candidates
        .iter()
        .map(|each| (each.name.as_str(), canonical_json(&each.server)))
        .collect()
}

/// What a running session does when the provided servers change: "Add a server: Claude Code connects
/// it when the updated settings arrive, without a restart"; "Change a server's entry: those sessions
/// reconnect to it with the new definition"; "Remove a server: a running interactive session
/// disconnects it once it reads the changed settings. A non-interactive (`-p`) run keeps it until it
/// ends." The host diffs by list, since events exist only on managed E2B (owner's note). Judgment: a
/// `-p` run applies none of the three, as "Interactive sessions that are already running apply your
/// edits".
#[must_use]
pub fn reconcile_managed_servers(
    previous: &[McpCandidate],
    next: &[McpCandidate],
    session: &ReconcileSession,
) -> ManagedServerChanges {
    if !session.interactive {
        return ManagedServerChanges::default();
    }
    let before = texts(previous);
    let after = texts(next);
    ManagedServerChanges {
        add: after
            .keys()
            .filter(|name| !before.contains_key(*name))
            .map(|name| (*name).to_owned())
            .collect(),
        change: after
            .iter()
            .filter(|(name, text)| before.get(*name).is_some_and(|earlier| earlier != *text))
            .map(|(name, _)| (*name).to_owned())
            .collect(),
        remove: before
            .keys()
            .filter(|name| !after.contains_key(*name))
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

crate::registry::exports! {
    "settings/mcp/managed-timing" {
        "MANAGED_DELIVERIES" => ok || MANAGED_DELIVERIES;
        "providedServersTiming" => ok |input: ProvidedTimingInput| provided_servers_timing(&input);
        "reconcileManagedServers" => ok |previous: Vec<McpCandidate>, next: Vec<McpCandidate>, session: ReconcileSession| reconcile_managed_servers(&previous, &next, &session);
    }
}
