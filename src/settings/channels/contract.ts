/** Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/contract.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { ChannelVerdict } from './verdict.ts';

const call = coreModule('settings/channels/contract');

/**
 * The channel contract a host reads from a server and applies to its events (channels-reference.md,
 * "Server options", "Notification format", "Gate inbound messages"; channels.md, "Security"). Pure
 * over data the host read from the MCP connection; the kit starts no server.
 */
/** What a server's capabilities declare. */
export interface ChannelCapabilities {
  /** `experimental['claude/channel']`: "Required. Always `{}`. Presence registers the notification listener." */
  readonly channel: boolean;
  /** `experimental['claude/channel/permission']`: `{}` declares relay; "omit the key or set it to `false`" opts out. */
  readonly permission: boolean;
  /** `tools`: "Two-way only", the standard MCP tool capability. */
  readonly tools: boolean;
}

/** Reads the `capabilities` object of the server's initialize result. */
export const channelCapabilities = (capabilities: unknown): ChannelCapabilities =>
  call('channelCapabilities', capabilities);

/** "To create a one-way channel, omit `capabilities.tools`": a channel with tools can take replies. */
export const isTwoWay = (capabilities: ChannelCapabilities): boolean =>
  call('isTwoWay', capabilities);

/**
 * `instructions` "Claude Code delivers ... to Claude as context when the server connects": the text
 * to add to the context, or `undefined` when the server gave none.
 */
export const channelInstructions = (instructions: unknown): string | undefined =>
  call('channelInstructions', instructions);

/** The `notifications/claude/channel` params. */
export interface ChannelNotification {
  readonly content: string;
  readonly meta?: Readonly<Record<string, string>>;
}

/**
 * The `notifications/claude/channel` params a server sent, read as a `ChannelNotification`: `content` is a
 * required string ("The event body"), so a notification without one is `undefined`; `meta` keeps the
 * entries whose key is an identifier and whose value is a string. Judgment (spec/judgments.md): the docs type
 * `meta` as `Record<string, string>` and say nothing about a value of another type; it is dropped, as a key
 * that is not an identifier is.
 */
export const readChannelNotification = (params: unknown): ChannelNotification | undefined =>
  call('readChannelNotification', params);

/**
 * The event as Claude sees it: `<channel source="..." key="value">content</channel>`. `source` "is set
 * automatically from your server's configured name"; each valid `meta` entry is an attribute.
 */
export const channelTag = (source: string, notification: ChannelNotification): string =>
  call('channelTag', source, notification);

/**
 * Whether an event reaches the session. "If the session hasn't loaded your server as a channel, or the
 * organization policy blocks it, Claude Code drops the events silently and returns no error": the
 * server must be named by a flag (`verdict` is set) and the verdict must admit it. Being in `.mcp.json`
 * alone is not enough (channels.md, "Security").
 */
export const deliversChannelEvents = (verdict: ChannelVerdict | undefined): boolean =>
  call('deliversChannelEvents', verdict);

/**
 * "Events queue into the session and are processed in order. If several notifications arrive while
 * Claude is busy, they're delivered together on the next turn and Claude handles them as a group."
 */
export const groupChannelEvents = (tags: readonly string[]): string | undefined =>
  call('groupChannelEvents', tags);

/** A message from a chat platform: who sent it and in which room. */
export interface InboundMessage {
  readonly from: { readonly id: string };
  readonly chat: { readonly id: string };
}

/**
 * "Gate on the sender's identity, not the chat or room identity": a message passes when its sender is
 * on the list, whatever room it came from.
 */
export const senderAllowed = (allowed: ReadonlySet<string>, message: InboundMessage): boolean =>
  call('senderAllowed', allowed, message);
