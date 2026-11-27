/**
 * The environment of a command hook (hooks.md, "Common input fields", "Exec form and shell form" and
 * "Persist environment variables"; env-vars.md; plugins/components.md, "Environment, quoting, and
 * matching MCP tools").
 *
 * - "A hook process inherits the parent environment, apart from the `OTEL_*` exporter variables that
 *   Claude Code removes from every subprocess it spawns". The parent environment is the host's with
 *   the settings files' `env` written over it (env-vars.md, "Precedence"), and with its credentials
 *   removed when `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is `1` (`scrub.ts`).
 * - Both forms "export [the path placeholders] as the environment variables `CLAUDE_PROJECT_DIR`,
 *   `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA`"; a plugin hook also gets
 *   `CLAUDE_PLUGIN_OPTION_<KEY>` for each option.
 * - What Claude Code sets in every subprocess it spawns: `CLAUDECODE` and `CLAUDE_CODE_CHILD_SESSION`
 *   (`1`), `CLAUDE_CODE_SESSION_ID` (the input's `session_id`) and `CLAUDE_PID` (its own process id).
 * - `CLAUDE_ENV_FILE` on the four events that have one, and only to a hook that runs no PowerShell;
 *   an inherited one never reaches another hook ("Other hook events don't have access to this variable").
 *
 * Judgment (spec/judgments.md): the exported variables replace an inherited one of the same name, and
 * the `OTEL_` prefix and the `CLAUDE_ENV_FILE` name are matched without regard to case on a Windows
 * host, whose environment names are case-insensitive, and exactly elsewhere.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/env.rs`, docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';
import type { HandlerEntry, RunContext } from '../context.ts';

const call = coreModule('hooks/handlers/command/env');

/**
 * The whole environment the process gets (the port adds nothing and removes nothing); `exported`
 * is what `resolveHookCommand` exports, resolved with the command's own substituter.
 */
export const processEnvironment = (
  entry: HandlerEntry<'command'>,
  exported: Readonly<Record<string, string>>,
  context: RunContext,
): Promise<Readonly<Record<string, string>>> =>
  call.async('processEnvironment', entry, exported, context);
