import type { Prompt, PromptArgument } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * MCP prompts as commands (mcp.md, "Use MCP prompts as commands", "Execute MCP prompts"). A connected
 * server's prompts are listed as `/servername:promptname (MCP)` and run by typing
 * `/mcp__servername__promptname` followed by space-separated arguments. Pure over the prompts the
 * host listed with `prompts/list`; `prompts/get` and injecting the result into the conversation are the
 * host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_prompts.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-prompts');

export type McpPromptDefinition = Pick<Prompt, 'name' | 'description'> & {
  readonly server: string;
  /** The prompt's defined parameters, in order. */
  readonly arguments?: readonly Pick<PromptArgument, 'name' | 'description' | 'required'>[];
};

/** The parameters of `prompts/get`. */
export interface McpPromptInvocation {
  readonly server: string;
  readonly prompt: string;
  readonly arguments: Readonly<Record<string, string>>;
}

export type McpPromptRead =
  | { readonly status: 'not-a-prompt' }
  | { readonly status: 'invoke'; readonly invocation: McpPromptInvocation }
  | { readonly status: 'refused'; readonly message: string };

export const listablePrompts = (prompts: readonly McpPromptDefinition[]): McpPromptDefinition[] =>
  call('listablePrompts', prompts);

/**
 * "Claude Code lists each MCP prompt as `/servername:promptname (MCP)`". Judgment (spec/judgments.md):
 * both names as configured and declared, since only the typed form is said to be normalised.
 */
export const mcpPromptListing = (server: string, prompt: string): string =>
  call('mcpPromptListing', server, prompt);

/**
 * "In the `/mcp__servername__promptname` form, Claude Code replaces any character in the server name
 * outside `A-Z`, `a-z`, `0-9`, `_`, and `-` with `_`, and uses the prompt name as the server declares it."
 */
export const mcpPromptCommand = (server: string, prompt: string): string =>
  call('mcpPromptCommand', server, prompt);

/**
 * "Pass them space-separated after the command. Claude Code splits the arguments on whitespace, so
 * each argument is a single token": no quoting. They go to the prompt's defined parameters in order.
 * Judgments (spec/judgments.md): a prompt takes no more arguments than it declares and needs its
 * required ones; two prompts with one command are answered by the first listed; the wording of a refusal.
 */
export const readMcpPromptInvocation = (
  input: string,
  prompts: readonly McpPromptDefinition[],
): McpPromptRead => call('readMcpPromptInvocation', input, prompts);
