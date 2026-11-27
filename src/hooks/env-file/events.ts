/**
 * hooks.md, "Persist environment variables": "`CLAUDE_ENV_FILE` is available for SessionStart, Setup,
 * CwdChanged, and FileChanged hooks. Other hook events don't have access to this variable". The
 * variables of `SessionStart` and `Setup` last the session; those of `CwdChanged` and `FileChanged`
 * "persist into subsequent Bash commands until the next CwdChanged event, when Claude Code clears them".
 * A hook that runs in PowerShell has no file either, and a hook in exec form runs no shell, so it is
 * not one.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/env-file/events.rs`, docs/RUST.md).
 */
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EnvFileKind } from './ports.ts';

const call = coreModule('hooks/env-file/events');

/** The file the hooks of this event append to, or none for an event that has no `CLAUDE_ENV_FILE`. */
export const envFileKindOf = (event: HookEvent): EnvFileKind | undefined =>
  call('envFileKindOf', event);
