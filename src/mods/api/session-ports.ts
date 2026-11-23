import type { JsonObject, JsonValue } from '../../foundation/index.ts';
import type { EventData } from './ports.ts';

/** The session port of the mods API (`$.session`) and what it answers; see `ports.ts`. */

/** One entry of `$.session.messages()`; a read with `last` or `after` carries each row's `uuid` (2.1.330). */
export interface SessionEntry {
  readonly role: string;
  readonly text: string;
  readonly toolUses: readonly JsonValue[];
  readonly uuid?: string;
}

/** What `$.session.messages({ last?, after? })` asked for (claude-code/index.d.ts:15081). */
export interface MessagesQuery {
  /** The newest entries, 1 to 4,096. */
  readonly last?: number;
  /** The `uuid` of the row to read on from: the entries after it. */
  readonly after?: string;
}

export interface ModSession {
  /**
   * Every entry of the transcript, oldest first, or those after the row `query.after`, each with
   * its `uuid` when `query` names `last` or `after`; the kit keeps the newest 4,096, or `last`.
   */
  messages(signal: AbortSignal, query?: MessagesQuery): Promise<readonly SessionEntry[]>;
  cwd(): Promise<string>;
  root(): Promise<string>;
  model(): Promise<string>;
  turns(): Promise<number>;
  id(): Promise<string>;
  repo(): Promise<JsonValue>;
  surfaces(): Promise<readonly string[]>;
  /** `{ startedAt, context: { tokens, window, percent }, rateLimits: [{ kind, percentUsed, resetsAt }], cost }`. */
  usage(): Promise<JsonObject>;
  /** The version, or `{ version, base?, builtAt? }` (claude-code/index.d.ts:3019). */
  version(): Promise<string | JsonObject>;
  /**
   * Compacts the conversation and answers what came of it: `{ messages, tokensBefore?,
   * tokensAfter?, usage? }`, or `{ skip }` (claude-code/index.d.ts:3031).
   */
  compact(signal: AbortSignal): Promise<JsonObject | void>;
  /**
   * Sends the message and resolves with `{ isDelivered }`. `refusal` is the reason the session takes
   * no new work from mods now (`RunGate.refusal`), when it takes none: a host whose target is a
   * subagent that finished or a teammate answers `{ isDelivered: false, reason }` with the sentence
   * from `unsentReason`, and still delivers to a subagent that is running.
   */
  send(to: string, text: string, signal: AbortSignal, refusal?: string): Promise<JsonObject>;
  /**
   * Appends `{ message, agentId? }` and answers the row as stored, `{ message, uuid }`
   * (claude-code/index.d.ts:11730), or `{ deny: reason }` when the host refuses it.
   */
  append(row: EventData, signal: AbortSignal): Promise<EventData | void>;
  authorize(request: EventData, signal: AbortSignal): Promise<unknown>;
}
