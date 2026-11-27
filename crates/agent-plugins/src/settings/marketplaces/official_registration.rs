//! Port of `src/settings/marketplaces/official-registration.ts`, switched.
//!
//! Whether Claude Code registers the official marketplace by itself (settings-reference.md, "Allow
//! only the official marketplace"; env-vars.md,
//! `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL`). Pure over facts the host keeps; the host
//! registers it and records what the verdict says to record.

use serde::{Deserialize, Serialize};

use super::registry::official_source;
use crate::registry::Port;

/// How this session was launched: an interactive terminal session, a `-p` run, a terminal attached
/// to a cloud session, or the VS Code extension. Only the first registers ("the first time you
/// start an interactive terminal session").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OfficialRegistrationLaunch {
    /// An interactive terminal session.
    Terminal,
    /// A `-p` run.
    Print,
    /// A terminal attached to a cloud session.
    CloudTerminal,
    /// The VS Code extension.
    VsCode,
}

/// `REGISTERS_AT`: whether a launch of this kind registers.
const fn registers_at(launch: OfficialRegistrationLaunch) -> bool {
    match launch {
        OfficialRegistrationLaunch::Terminal => true,
        OfficialRegistrationLaunch::Print
        | OfficialRegistrationLaunch::CloudTerminal
        | OfficialRegistrationLaunch::VsCode => false,
    }
}

/// The facts the host keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialRegistrationFacts {
    /// How this session was launched.
    pub launch: OfficialRegistrationLaunch,
    /// The official marketplace is already registered on this machine: it is kept.
    pub registered: bool,
    /// The host recorded a blocked attempt or a skip before: "doesn't retry after the policy
    /// changes".
    pub skip_recorded: bool,
    /// `officialMarketplaceAutoinstallEnabled(env)`: `false` "skips the registration permanently".
    pub autoinstall_enabled: bool,
}

/// What the host does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum OfficialRegistration {
    /// Already registered, or not this kind of launch: nothing happens and nothing is recorded.
    None,
    /// The host registers `claude-plugins-official` from its GitHub repository.
    Register,
    /// The policy blocks the marketplace or the variable is set: the host records the skip and
    /// never registers it by itself again; a managed `extraKnownMarketplaces` entry, an
    /// `enabledPlugins` entry for one of its plugins or `claude plugin marketplace add
    /// anthropics/claude-plugins-official` still registers it.
    RecordSkip,
}

/// The gate's verdict, as this module reads it.
#[derive(Deserialize)]
struct Checked {
    allowed: bool,
}

/// The verdict; `gate` is the host's `MarketplaceGate`, asked `check` of the official source.
pub async fn official_marketplace_registration(
    gate: &Port,
    facts: &OfficialRegistrationFacts,
) -> OfficialRegistration {
    if facts.registered || facts.skip_recorded || !registers_at(facts.launch) {
        return OfficialRegistration::None;
    }
    if !facts.autoinstall_enabled
        || !gate
            .method::<Checked>("check", vec![official_source()])
            .await
            .allowed
    {
        return OfficialRegistration::RecordSkip;
    }
    OfficialRegistration::Register
}

crate::registry::exports! {
    "settings/marketplaces/official-registration" {
        "officialMarketplaceRegistration" => async ok |gate: Port, facts: OfficialRegistrationFacts|
            official_marketplace_registration(&gate, &facts).await;
    }
}
