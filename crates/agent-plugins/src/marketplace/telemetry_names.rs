//! Port of `src/marketplace/telemetry-names.ts`, switched.
//!
//! The plugin and marketplace names an OpenTelemetry event reports (plugins/security.md, "Find plugins
//! in telemetry"; plugins/measure.md, "Redacted plugin names in your backend"). Pure: the host says
//! whether the plugin is from the official tier and whether `OTEL_LOG_TOOL_DETAILS=1`.

use serde::{Deserialize, Serialize};

/// The events that carry plugin names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryEvent {
    /// `plugin_loaded`.
    #[serde(rename = "plugin_loaded")]
    PluginLoaded,
    /// `plugin_installed`.
    #[serde(rename = "plugin_installed")]
    PluginInstalled,
    /// `skill_activated`.
    #[serde(rename = "skill_activated")]
    SkillActivated,
}

/// The events that carry plugin names.
pub const PLUGIN_TELEMETRY_EVENTS: [TelemetryEvent; 3] = [
    TelemetryEvent::PluginLoaded,
    TelemetryEvent::PluginInstalled,
    TelemetryEvent::SkillActivated,
];

/// What decides which names an event reports.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryNameFacts {
    /// The plugin is from an official-tier marketplace: its names are reported as they are.
    pub official: bool,
    /// `OTEL_LOG_TOOL_DETAILS=1` on the machine that exports.
    pub log_tool_details: bool,
    /// The plugin.
    pub plugin: String,
    /// Its marketplace.
    pub marketplace: String,
}

/// The names an event reports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReportedNames {
    /// `plugin.name`.
    #[serde(rename = "plugin.name", skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    /// `marketplace.name`.
    #[serde(rename = "marketplace.name", skip_serializing_if = "Option::is_none")]
    pub marketplace_name: Option<String>,
}

fn real(facts: &TelemetryNameFacts) -> ReportedNames {
    ReportedNames {
        plugin_name: Some(facts.plugin.clone()),
        marketplace_name: Some(facts.marketplace.clone()),
    }
}

/// The literal `third-party` replaces the names on `plugin_loaded`.
fn literal() -> ReportedNames {
    ReportedNames {
        plugin_name: Some("third-party".to_owned()),
        marketplace_name: Some("third-party".to_owned()),
    }
}

/// What a non-official plugin's event reports by default, per event: `plugin_loaded` redacts, the others omit.
fn redacted(event: TelemetryEvent) -> ReportedNames {
    match event {
        TelemetryEvent::PluginLoaded => literal(),
        TelemetryEvent::PluginInstalled | TelemetryEvent::SkillActivated => ReportedNames {
            plugin_name: None,
            marketplace_name: None,
        },
    }
}

/// The names the event carries: real for the official tier or with `OTEL_LOG_TOOL_DETAILS=1`, else redacted or omitted.
#[must_use]
pub fn reported_names(event: TelemetryEvent, facts: &TelemetryNameFacts) -> ReportedNames {
    if facts.official || facts.log_tool_details {
        real(facts)
    } else {
        redacted(event)
    }
}

crate::registry::exports! {
    "marketplace/telemetry-names" {
        "PLUGIN_TELEMETRY_EVENTS" => ok || PLUGIN_TELEMETRY_EVENTS;
        "reportedNames" => ok |event: TelemetryEvent, facts: TelemetryNameFacts| reported_names(event, &facts);
    }
}
