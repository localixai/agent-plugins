/**
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/adapters/core_profiles_source.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('convert/adapters/core-profiles-source');

/**
 * The tool profiles of the core (Agent Plugins Extended §18.2.7; core §7.3.6, §7.3.7) as JavaScript
 * text, part of the program `run-core-handler.mjs`: the same table as `hooks/core/profiles.ts` and
 * `hooks/core/profile-results.ts`, which the adapter cannot import since it runs where the kit is not
 * installed. A hook never sees the facts only a client holds, so where the Claude Code fields do not
 * determine a value the profile is omitted (J18.16, J18.21): a shell command's exit code on
 * `PostToolUse` is `0` only without `returnCodeInterpretation`, an MCP tool's declared name is read
 * only when `__` occurs once after `mcp__`, an MCP call's completed profile is omitted (no
 * `CallToolResult`), and a text or notebook read gets no profile (the text Claude received is never
 * rebuilt from `file.content`).
 */
export const CORE_PROFILES_SOURCE: string = call('CORE_PROFILES_SOURCE');
