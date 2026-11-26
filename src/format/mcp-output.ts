import { coreModule } from '../wasm/bridge.ts';
import type { McpCallToolResult } from '../protocol/official.ts';
import type { McpTransport } from './mcp-config.ts';
import type { EnvLookup } from './mcp-env.ts';
import type { McpToolHints } from './mcp-tool.ts';
import { estimateTokens } from './token-estimate.ts';

/**
 * How large an MCP tool result may be and what happens to one that is too large (mcp.md, "MCP output
 * limits and warnings", "Raise the limit for a specific tool", "Images in tool results"; env-vars.md,
 * `MAX_MCP_OUTPUT_TOKENS`). Pure over data: saving a file, showing a warning and counting tokens with
 * a real tokenizer are the host's, handed in or done with the verdict.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_output.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-output');

export interface McpOutputLimits {
  /** `MAX_MCP_OUTPUT_TOKENS`, or 25,000. */
  readonly maxTokens: number;
  readonly warnTokens: number;
  /** The character threshold for text: the tool's own `anthropic/maxResultSizeChars`, or 50,000. */
  readonly textChars: number;
  /** The tool declares its own limit: "that value instead for text content, regardless of" the variable. */
  readonly declared: boolean;
  readonly countTokens: (text: string) => number;
}

/**
 * The limits for one tool. "The environment variable applies to tools that don't declare their own
 * limit"; a tool that does uses its value for text, and its images still meet `MAX_MCP_OUTPUT_TOKENS`.
 */
export const mcpOutputLimits = (
  lookup: EnvLookup,
  hints: Pick<McpToolHints, 'maxResultSizeChars'> = {},
  countTokens: (text: string) => number = estimateTokens,
): McpOutputLimits => call('mcpOutputLimits', lookup, hints, countTokens);

export type McpResultVerdict =
  | {
      readonly action: 'inline';
      readonly tokens: number;
      /** The output passed the warning threshold. */
      readonly warn: boolean;
      /** Image content makes the result subject to the token limit and it is over it; the host decides. */
      readonly exceedsTokenLimit: boolean;
    }
  | {
      readonly action: 'persist';
      /** The text to save to a file; the conversation gets `persistedResultMessage(path)` instead. */
      readonly text: string;
      readonly tokens: number;
      readonly warn: boolean;
    }
  | {
      readonly action: 'error';
      /** What Claude receives as the tool's error message. */
      readonly message: string;
      /** Characters cut out between the kept ends. */
      readonly removed: number;
    };

/**
 * The text of an error as Claude receives it: "Error text longer than about 11,000 characters keeps only
 * its first 5,000 and last 5,000 characters". For an error that is not a tool's result (a failed call),
 * so a host bounds both the same way.
 */
export const boundedMcpErrorText = (text: string): string => call('boundedMcpErrorText', text);

/**
 * What to do with a tool result. An error result is exempt from the limits and has its text bounded.
 * A successful one with no image content is saved when it is over the limit; one with image content is
 * never saved ("a successful result with no image content") and reports whether it is over the token
 * limit. The images' data counts towards the tokens (judgment, spec/judgments.md).
 */
export const limitMcpResult = (
  result: McpCallToolResult,
  limits: McpOutputLimits,
): McpResultVerdict => call('limitMcpResult', result, limits);

/** What replaces a saved result in the conversation: "a message that names the file path" (wording is the kit's). */
export const persistedResultMessage = (path: string): string =>
  call('persistedResultMessage', path);

/** What tells Claude where the original of an image result is saved (wording is the kit's). */
export const imageResultNote = (path: string): string => call('imageResultNote', path);

export interface McpImageFile {
  readonly mimeType: string;
  readonly extension: string;
  /** The base64 data as the server sent it: "the original bytes". */
  readonly data: string;
}

/**
 * The image files to save beside the inline copy. With session persistence off
 * (`--no-session-persistence`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY`) "Claude Code writes no image file".
 */
export const mcpImageFiles = (result: McpCallToolResult, persist: boolean): McpImageFile[] =>
  call('mcpImageFiles', result, persist);

/** What a transport does with a message over its bound. */
export type McpOversize = 'fail-request' | 'close-connection';

/** What happens to a message of `bytes` (after decompression) from a server of `transport`: `undefined` when it is fine. */
export const oversizedMcpMessage = (
  transport: McpTransport,
  bytes: number,
): McpOversize | undefined => call('oversizedMcpMessage', transport, bytes);
