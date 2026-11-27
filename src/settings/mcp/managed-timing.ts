/** Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/managed_timing.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { McpCandidate } from './scope.ts';

const call = coreModule('settings/mcp/managed-timing');

/**
 * How `managedMcpServers` reaches the machine (managed-mcp.md, "When provided servers connect"):
 * through server-managed settings, which are fetched, through a Claude apps gateway sign-in, "Claude
 * Code loads the policy before the session starts", or from the device itself (MDM, registry,
 * `managed-settings.json`), judged to be read before the session too (spec/judgments.md).
 */
export const MANAGED_DELIVERIES: readonly ['serverManaged', 'gateway', 'device'] =
  call('MANAGED_DELIVERIES');

export type ManagedDelivery = (typeof MANAGED_DELIVERIES)[number];

export interface ProvidedTimingInput {
  readonly delivery: ManagedDelivery;
  /** An interactive session; `false` for a `claude -p` run. */
  readonly interactive: boolean;
  /** The machine has a cached copy of the server-managed settings. */
  readonly cached: boolean;
  /** The server did not confirm the settings for this session. */
  readonly confirmationFailed: boolean;
}

export interface ProvidedTiming {
  /** MCP servers wait for the server's confirmation before they load. */
  readonly waitsForConfirmation: boolean;
  /** The provided servers are part of the session's first connection. */
  readonly connectsAtStart: boolean;
  /** An interactive first launch connects them as soon as the settings arrive. */
  readonly connectsOnArrival: boolean;
  /** A `claude -p` run that has already started "can finish without them". */
  readonly mayFinishWithout: boolean;
  /** What `/status` says when they are withheld. */
  readonly withheld: string | undefined;
}

/** When the provided servers connect in a session that starts (managed-mcp.md, "When provided servers connect"). */
export const providedServersTiming = (input: ProvidedTimingInput): ProvidedTiming =>
  call('providedServersTiming', input);

export interface ManagedServerChanges {
  readonly add: readonly string[];
  readonly change: readonly string[];
  readonly remove: readonly string[];
}

/**
 * What a running session does when the provided servers change: "Add a server: Claude Code connects
 * it when the updated settings arrive, without a restart"; "Change a server's entry: those sessions
 * reconnect to it with the new definition"; "Remove a server: a running interactive session
 * disconnects it once it reads the changed settings. A non-interactive (`-p`) run keeps it until it
 * ends." The host diffs by list, since events exist only on managed E2B (owner's note). Judgment: a
 * `-p` run applies none of the three, as "Interactive sessions that are already running apply your
 * edits".
 */
export const reconcileManagedServers = (
  previous: readonly McpCandidate[],
  next: readonly McpCandidate[],
  session: { readonly interactive: boolean },
): ManagedServerChanges => call('reconcileManagedServers', previous, next, session);
