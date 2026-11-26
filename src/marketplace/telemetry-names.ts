import { coreModule } from '../wasm/bridge.ts';

/**
 * The plugin and marketplace names an OpenTelemetry event reports (plugins/security.md, "Find plugins
 * in telemetry"; plugins/measure.md, "Redacted plugin names in your backend"). Pure: the host says
 * whether the plugin is from the official tier and whether `OTEL_LOG_TOOL_DETAILS=1`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/telemetry_names.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/telemetry-names');

export const PLUGIN_TELEMETRY_EVENTS: readonly [
  'plugin_loaded',
  'plugin_installed',
  'skill_activated',
] = call('PLUGIN_TELEMETRY_EVENTS');
export type TelemetryEvent = (typeof PLUGIN_TELEMETRY_EVENTS)[number];

export interface TelemetryNameFacts {
  /** The plugin is from an official-tier marketplace: its names are reported as they are. */
  readonly official: boolean;
  /** `OTEL_LOG_TOOL_DETAILS=1` on the machine that exports. */
  readonly logToolDetails: boolean;
  readonly plugin: string;
  readonly marketplace: string;
}

export interface ReportedNames {
  readonly 'plugin.name'?: string;
  readonly 'marketplace.name'?: string;
}

/** The names the event carries: real for the official tier or with `OTEL_LOG_TOOL_DETAILS=1`, else redacted or omitted. */
export const reportedNames = (event: TelemetryEvent, facts: TelemetryNameFacts): ReportedNames =>
  call('reportedNames', event, facts);
