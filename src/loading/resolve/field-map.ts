import { coreModule } from '../../wasm/bridge.ts';
import type { PluginVariable } from './variables.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/field_map.rs`, docs/RUST.md). */
const call = coreModule('loading/resolve/field-map');

/**
 * The table "Where each variable resolves" (plugins/manifest-reference.md): per plugin component,
 * the fields where `${...}` resolves inline and what the process receives in its environment.
 * Each resolver reads its field list here, so a component Claude Code adds is one row.
 */
export const VARIABLE_COMPONENTS: readonly [
  'hook-command',
  'monitor-command',
  'mcp-stdio',
  'mcp-remote',
  'lsp',
  'markdown-content',
] = call('VARIABLE_COMPONENTS');
export type VariableComponent = (typeof VARIABLE_COMPONENTS)[number];

/** A field `${...}` resolves in; `body` is "anywhere in the Markdown body". */
export type VariableField =
  | 'command'
  | 'args'
  | 'env'
  | 'url'
  | 'headers'
  | 'headersHelper'
  | 'workspaceFolder'
  | 'body';

/** The name of the option variables, `CLAUDE_PLUGIN_OPTION_<KEY>`, as the table writes it. */
export const OPTION_VARIABLES: 'CLAUDE_PLUGIN_OPTION_<KEY>' = call('OPTION_VARIABLES');
export type ExportedVariable = PluginVariable | typeof OPTION_VARIABLES;

export interface ComponentVariables {
  readonly fields: readonly VariableField[];
  readonly exported: readonly ExportedVariable[];
}

export const VARIABLE_FIELDS: { readonly [C in VariableComponent]: ComponentVariables } =
  call('VARIABLE_FIELDS');
