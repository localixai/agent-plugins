/** Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/startup_failure.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { StartupFetchFacts } from './applicability.ts';

const call = coreModule('settings/managed/startup-failure');

/**
 * What a failed startup fetch of server-managed settings does (server-managed-settings.md, "Enforce
 * fail-closed startup"; errors.md, "Cloud gateway session expired"). Pure over the facts the host
 * knows; the host prints the line, exits or opens the session.
 */
/** The line a gateway session that answered `401` prints before it opens signed out. */
export const GATEWAY_SESSION_EXPIRED: 'Cloud gateway session expired — run /login to reconnect.' =
  call('GATEWAY_SESSION_EXPIRED');

export interface StartupFailureFacts extends StartupFetchFacts {
  /** An attended interactive launch: `claude` in a terminal, not `-p`, the SDK or a background run. */
  readonly attendedInteractive: boolean;
  /** The HTTP status the gateway answered with; absent when it could not be reached. */
  readonly gatewayStatus?: number;
}

export type StartupFailure =
  /** The session starts anyway, on a cached or absent policy. */
  | { readonly outcome: 'starts' }
  /** The session exits with an error. */
  | { readonly outcome: 'exits' }
  /** The host prints `message` and opens the session signed out of the gateway until `/login`. */
  | { readonly outcome: 'opens-signed-out'; readonly message: string };

/**
 * "If the gateway answers an attended interactive launch with a `401` and this setting is off, the
 * gateway has ended that sign-in": the line, then a session signed out of the gateway. "When the fetch
 * fails in any other way, or in any other kind of launch except a `claude auth` subcommand, the client
 * exits with an error." Outside a gateway, `forceRemoteSettingsRefresh` or Anthropic's requirement
 * exits and anything else starts.
 */
export const startupFetchFailure = (facts: StartupFailureFacts): StartupFailure =>
  call('startupFetchFailure', facts);
