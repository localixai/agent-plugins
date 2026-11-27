/** Implemented by the Rust core (`crates/agent-plugins/src/settings/general/edit_effect.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { FileScope } from './reach.ts';

const call = coreModule('settings/general/edit-effect');

/**
 * When an edit reaches a running session (settings.md, "When edits take effect"; managed-settings.md,
 * "Where and when a policy applies"). A pure decision: the host watches the files, reloads and fires
 * the `ConfigChange` hook; this says what each change is owed.
 */
/** How the change arrived: a settings file Claude Code watches, or a managed source delivered on a schedule. */
export type Arrival = 'file' | 'mdm' | 'server-managed';

export interface SettingsEdit {
  readonly scope: FileScope;
  readonly arrival: Arrival;
  /** The top-level key, or the dotted path of one inside a block; `undefined` for a change to the file as a whole. */
  readonly key?: string;
  /** The edit removes the key rather than setting it. */
  readonly removed?: boolean;
}

export type Applies =
  /** The running session reloads it: "most edits ... without a restart". */
  | 'now'
  /** A managed source reaches the session "on a schedule rather than on save". */
  | 'on-schedule'
  /** "reads some keys only once, at session start". */
  | 'next-session'
  /** The running session keeps the old behaviour until each session restarts. */
  | 'after-restart';

export interface EditEffect {
  readonly applies: Applies;
  /** Claude Code runs the `ConfigChange` hook "for each settings-file change it detects, not for managed settings that arrive from MDM or the claude.ai console". */
  readonly configChangeHook: boolean;
}

/** What an edit is owed: the arrival's effect, unless the key waits for a restart. */
export const editEffect = (edit: SettingsEdit): EditEffect => call('editEffect', edit);

export const loadsFileCreatedMidSession = (
  scope: FileScope,
  folderExistedAtStart: boolean,
): boolean => call('loadsFileCreatedMidSession', scope, folderExistedAtStart);

export interface ManagedChangeFacts {
  /** The session has restarted since the administrator changed the managed source. */
  readonly restarted: boolean;
  /** The managed source the administrator changed, as `/status` names sources. */
  readonly changedSource: string;
  /** The managed source `Setting sources` of `/status` names for the session. */
  readonly statusSource: string;
}

/** What to tell a person whose managed change has not reached them. */
export type ManagedChangeAdvice =
  /** Managed sources "reach a running session on the schedule in the delivery table, so restart the session first". */
  | 'restart'
  /** `/status` names another source than the one changed: "a higher-priority source applies". */
  | 'higher-source-applies'
  /** The session runs the changed source: the change has reached it. */
  | 'reached';

/**
 * settings.md, "A managed change hasn't reached you": restart first; "If `/status` then names a
 * different source than the one your administrator changed, a higher-priority source applies" (the
 * order among managed sources is managed-settings.md, "How Claude Code combines managed sources",
 * the host's to show).
 */
export const managedChangeAdvice = (facts: ManagedChangeFacts): ManagedChangeAdvice =>
  call('managedChangeAdvice', facts);
