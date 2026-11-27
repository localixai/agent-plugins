/**
 * Permission relay's request side (channels-reference.md, "Relay permission prompts"): the IDs Claude Code
 * issues, the four fields of a request, and the rule that applies a verdict only when its ID matches an
 * open request.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/relay.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/channels/relay');

/** The `notifications/claude/channel/permission_request` params. */
export interface PermissionRequestParams {
  readonly request_id: string;
  readonly tool_name: string;
  readonly description: string;
  readonly input_preview: string;
}

/** The `notifications/claude/channel/permission` params a channel sends back. */
export interface PermissionVerdictParams {
  readonly request_id: string;
  readonly behavior: 'allow' | 'deny';
}

export interface RelayToolCall {
  readonly toolName: string;
  readonly description?: string;
  readonly input: Readonly<Record<string, unknown>>;
}

export type RelayOutcome =
  | { readonly applied: true; readonly behavior: 'allow' | 'deny' }
  | { readonly applied: false };

export interface PermissionRelay {
  /** Opens a request for a permission prompt and returns what to notify the channel with. */
  open(call: RelayToolCall): PermissionRequestParams;
  /** A verdict from a channel: applied "only if the ID matches an open request". */
  verdict(params: PermissionVerdictParams): RelayOutcome;
  /** Someone answered at the terminal: "that answer is applied instead and the pending remote request is dropped". */
  answerLocally(requestId: string): void;
}

/** `random` returns a number in [0, 1), the host's source of randomness (L3). */
export const createPermissionRelay = (random: () => number): PermissionRelay =>
  call('createPermissionRelay', random);
