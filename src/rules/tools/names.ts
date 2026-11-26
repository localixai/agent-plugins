/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/names.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/tools/names');

/** The tool name an entry of a tool list or flag spells: `Bash(git push *)` names `Bash`. */
export const toolNameOfEntry = (entry: string): string => call('toolNameOfEntry', entry);

/** An MCP tool: `mcp__<server>__<tool>`. */
export const isMcpToolName = (toolName: string): boolean => call('isMcpToolName', toolName);
