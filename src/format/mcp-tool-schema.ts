import type { Tool } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What Claude Code does with a tool's input schema before the tool goes to the API (mcp.md, "Tool
 * input schemas with a root-level combinator", "Tools with invalid input schemas", "Tools whose input
 * schema the API refuses"): a root `anyOf`, `oneOf` or `allOf` is flattened into one object, a tool
 * whose schema the API would refuse is excluded (and Claude is told), and a tool the API did refuse
 * is declared without parameters. Pure over data; the log is returned, never written.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_tool_schema.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-tool-schema');

export type McpToolDeclaration = Pick<Tool, 'name' | 'description'> & {
  readonly inputSchema: Readonly<Record<string, unknown>>;
};

/** A tool as a server listed it: its input schema is whatever the server sent, checked here. */
export type McpListedTool = Pick<Tool, 'name' | 'description'> & {
  readonly inputSchema: unknown;
};

export interface McpToolPolicy {
  /** The remote configuration that enables the rewrite of root-level combinators was received. */
  readonly rewriteRootCombinators: boolean;
  /**
   * The checks of "Tools with invalid input schemas" exclude the tool: false "when feature flag
   * fetching is off or the flags have never arrived" (the tool is sent and the log records it).
   */
  readonly excludeInvalid: boolean;
}

export type McpToolPreparation =
  | { readonly sent: true; readonly tool: McpToolDeclaration; readonly log: readonly string[] }
  | { readonly sent: false; readonly reasons: readonly string[]; readonly log: readonly string[] };

/**
 * The tool as it goes to the API, or why it does not. Judgments (spec/judgments.md): a branch that is
 * not an object schema, a `$ref` branch or an empty combinator cannot be flattened and skips the tool;
 * properties two branches define differently are kept nested in an `anyOf`; the wording of the log
 * lines other than the phrases the docs quote.
 */
export const prepareMcpTool = (listed: McpListedTool, policy: McpToolPolicy): McpToolPreparation =>
  call('prepareMcpTool', listed, policy);

/**
 * What Claude is told about the tools Claude Code excluded: "tells Claude which tools it excluded and
 * why, so you can ask Claude why a tool is missing". The wording is the kit's. `undefined` for none.
 */
export const excludedToolsNotice = (
  tools: readonly { readonly name: string; readonly reasons: readonly string[] }[],
): string | undefined => call('excludedToolsNotice', tools);

/**
 * The tool the API refused, resent "with the tool declared without parameters", and the line for the
 * server's log: "the API refused this tool's input schema ... The log line ends with `The API said:`
 * and the API's reason".
 */
export const refusedSchemaFallback = (
  tool: McpToolDeclaration,
  apiReason: string,
): { readonly tool: McpToolDeclaration; readonly log: string } =>
  call('refusedSchemaFallback', tool, apiReason);
