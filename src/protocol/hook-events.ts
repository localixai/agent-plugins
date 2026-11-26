/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/hook_events.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { HookEvent, HookInput, SyncHookJSONOutput } from './official.ts';
import type { Assert, Equal } from './type-equality.ts';

const call = coreModule('protocol/hook-events');

/**
 * Every hook event Claude Code fires, in its documented order (hooks.md, "Hook events"). The list
 * exists because a host needs the events at runtime; its members must equal the SDK's
 * `HookEvent` exactly, which the assertion below checks at compile time.
 */
export const HOOK_EVENTS: readonly [
  'PreToolUse',
  'PostToolUse',
  'PostToolUseFailure',
  'PostToolBatch',
  'Notification',
  'UserPromptSubmit',
  'UserPromptExpansion',
  'SessionStart',
  'SessionEnd',
  'Stop',
  'StopFailure',
  'SubagentStart',
  'SubagentStop',
  'PreCompact',
  'PostCompact',
  'PreModelSwitch',
  'PostModelSwitch',
  'PermissionRequest',
  'PermissionDenied',
  'Setup',
  'TeammateIdle',
  'TaskCreated',
  'TaskCompleted',
  'Elicitation',
  'ElicitationResult',
  'ConfigChange',
  'WorktreeCreate',
  'WorktreeRemove',
  'InstructionsLoaded',
  'CwdChanged',
  'FileChanged',
  'DirectoryAdded',
  'MessageDisplay',
] = call('HOOK_EVENTS');

/** The input Claude Code sends a hook on event `E`. */
export type HookInputOf<E extends HookEvent> = Extract<HookInput, { hook_event_name: E }>;

/** The event-specific part of a hook's JSON output, for the events that have one. */
export type HookSpecificOutput = NonNullable<SyncHookJSONOutput['hookSpecificOutput']>;

export type HookSpecificOutputOf<E extends HookEvent> = Extract<
  HookSpecificOutput,
  { hookEventName: E }
>;

/** Events whose JSON output has an event-specific part. */
export type EventWithSpecificOutput = HookSpecificOutput['hookEventName'];

/** @public Drift guard: the list names every official event and nothing else. */
export type HookEventsMatchSdk = Assert<Equal<(typeof HOOK_EVENTS)[number], HookEvent>>;

/** @public Drift guard: every official event has an official input type. */
export type HookInputsCoverEvents = Assert<Equal<HookInput['hook_event_name'], HookEvent>>;
