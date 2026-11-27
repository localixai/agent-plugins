/**
 * The three path variables Claude Code provides to plugin components (plugins/manifest-reference.md,
 * "Environment variables"). `${CLAUDE_PLUGIN_ROOT}` is the installed version, `${CLAUDE_PLUGIN_DATA}`
 * the persistent folder, `${CLAUDE_PROJECT_DIR}` the project root.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/variables.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { SubstitutionScope } from './scope.ts';

const call = coreModule('loading/resolve/variables');

export const PLUGIN_VARIABLES: readonly [
  'CLAUDE_PLUGIN_ROOT',
  'CLAUDE_PLUGIN_DATA',
  'CLAUDE_PROJECT_DIR',
] = call('PLUGIN_VARIABLES');
export type PluginVariable = (typeof PLUGIN_VARIABLES)[number];

/** The variable's path as the host reports it: what a process receives in its environment. */
export const pluginVariableValue = (
  variable: PluginVariable,
  scope: SubstitutionScope,
): Promise<string> => call.async('pluginVariableValue', variable, scope);
