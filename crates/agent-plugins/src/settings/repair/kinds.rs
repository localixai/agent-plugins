//! Ported from `src/settings/repair/kinds.ts`; only the core uses it now, so it has no wrapper.
//!
//! What to do with a value that does not have the type its key documents. One strategy per kind,
//! so what differs per key is a row of `KEY_SPECS`: the policy tier fails closed where the docs say
//! so (managed-settings.md, "Find entries Claude Code dropped"), every other source loses the value
//! or the entry. The kinds are `RepairKind` (`settings/key_spec.rs`, their first use in the core).

use crate::js::JsValue;
use crate::settings::key_spec::RepairKind;
use crate::settings::mcp::entries::{allowed_mcp_servers, denied_mcp_servers};
use crate::settings::permissions::auto_mode::{auto_mode_block, consent_boolean};
use crate::settings::permissions::read::{disable_value, permissions_block};
use crate::settings::repair::closed::{
    as_written, gateway_networks, match_mode, model_denylist, org_uuid, provider_allowlist,
};
use crate::settings::repair::hooks::hooks;
use crate::settings::repair::lists::{
    blocked_sources, channel_allowlist, closed_string_list, strict_sources, string_list,
};
use crate::settings::repair::support::{RepairAt, Repaired};
use crate::settings::repair::values::{
    boolean, enabled_plugins, hooks_switch, lock_boolean, marketplace_map, plugin_only, record,
    skill_overrides, string, workflow_size,
};

/// `asWritten` takes the value alone.
fn as_written_at(value: &JsValue, _at: &RepairAt) -> Repaired {
    as_written(value.clone())
}

/// `REPAIRS[kind]`: the strategy of each kind, in the table's order.
#[must_use]
pub fn repair_of(kind: RepairKind) -> fn(&JsValue, &RepairAt) -> Repaired {
    match kind {
        RepairKind::Boolean => boolean,
        RepairKind::LockBoolean => lock_boolean,
        RepairKind::HooksSwitch => hooks_switch,
        RepairKind::String => string,
        RepairKind::StringList => string_list,
        RepairKind::ClosedStringList => closed_string_list,
        RepairKind::ChannelAllowlist => channel_allowlist,
        RepairKind::AllowedMcpServers => allowed_mcp_servers,
        RepairKind::DeniedMcpServers => denied_mcp_servers,
        RepairKind::StrictSources => strict_sources,
        RepairKind::BlockedSources => blocked_sources,
        RepairKind::EnabledPlugins => enabled_plugins,
        RepairKind::MarketplaceMap => marketplace_map,
        RepairKind::Record => record,
        RepairKind::SkillOverrides => skill_overrides,
        RepairKind::WorkflowSize => workflow_size,
        RepairKind::PluginOnly => plugin_only,
        RepairKind::Hooks => hooks,
        RepairKind::AsWritten => as_written_at,
        RepairKind::ProviderAllowlist => provider_allowlist,
        RepairKind::MatchMode => match_mode,
        RepairKind::ModelDenylist => model_denylist,
        RepairKind::OrgUuid => org_uuid,
        RepairKind::GatewayNetworks => gateway_networks,
        RepairKind::Permissions => permissions_block,
        RepairKind::AutoMode => auto_mode_block,
        RepairKind::DisableMode => disable_value,
        RepairKind::ConsentBoolean => consent_boolean,
    }
}
