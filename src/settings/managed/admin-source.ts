/**
 * The managed sources and what a machine can hold in each (managed-settings.md, "Choose a delivery
 * mechanism", "How Claude Code combines managed sources", "When an admin document counts as
 * present", "Find entries Claude Code dropped"). The kit reads no file, plist or registry: the host
 * says what it found at each source (`SourceRead`) and this module decides what that document is.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/admin_source.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/managed/admin-source');

export const MANAGED_SOURCES: readonly ['remote', 'plist', 'hklm', 'file', 'hkcu'] =
  call('MANAGED_SOURCES');
export type ManagedSource = (typeof MANAGED_SOURCES)[number];

/** Why a document exists and could not be read: "the operating system denied the read" or any other failure. */
export const UNREADABLE_CAUSES: readonly ['denied', 'failed'] = call('UNREADABLE_CAUSES');
export type UnreadableCause = (typeof UNREADABLE_CAUSES)[number];

/** A source that exists and could not be read: the cause and the underlying error's text (errors.md, `Detail: <source>: <reason>`). */
export interface UnreadableRead {
  readonly unreadable: UnreadableCause;
  readonly reason: string;
}

/**
 * What the host found at one source. `undefined`: nothing there (not a failure). `value`: the
 * content, either the parsed JSON or its text (a plist the host converted with `plutil` comes as
 * the converted JSON). `unreadable`: it exists and the read failed, with the error. `malformed`: the
 * platform converter (`plutil`) reported the document malformed, with its message.
 */
export type SourceRead =
  | undefined
  | { readonly value: unknown }
  | UnreadableRead
  | { readonly malformed: string };

/** The control keys: neither counts as a policy key (managed-settings.md, "How Claude Code combines managed sources"). */
export const CONTROL_KEYS: readonly string[] = call('CONTROL_KEYS');

/** The forms a stored policy takes, each with what makes it unparseable (managed-settings.md, "Find entries Claude Code dropped"). */
type Form = 'payload' | 'jsonFile' | 'plist' | 'registryValue';

export interface SourceTraits {
  /** A higher rank is read first. Equal ranks are alternatives for one OS (a plist or the HKLM key), never both. */
  readonly rank: number;
  /** "Admin source: one of the first three sources below. The HKCU registry is user-writable and isn't one." */
  readonly admin: boolean;
  readonly form: Form;
  /** The policy is stored on the machine ("endpoint-managed"), not fetched. */
  readonly onMachine: boolean;
  /** How `/status` names it in `Setting sources`. */
  readonly statusLabel: string;
}

export const SOURCE_TRAITS: { readonly [S in ManagedSource]: SourceTraits } = call('SOURCE_TRAITS');

/** One source after reading: what it delivers and whether it counts. */
export interface AdminDocument {
  readonly source: ManagedSource;
  /** What the file says, repaired as a managed document is (`readSettingsLayer`); `{}` when absent, unreadable or broken. */
  readonly settings: Settings;
  /** The parsed object before repair, for the control keys and the presence rule. */
  readonly raw: Readonly<Record<string, unknown>>;
  /** The keys the document carries with a value no repair could keep (`PolicyLayer.invalid`). */
  readonly invalid: readonly string[];
  /** "A document is present when it sets any policy key to a value other than null", or exists and cannot be read. */
  readonly present: boolean;
  /** It carries at least one policy key, so it can be the policy. */
  readonly delivers: boolean;
  readonly unreadable?: UnreadableCause;
  /** Where and why it could not be read: the label a report names the source by, and the error. */
  readonly unreadableAt?: { readonly label: string; readonly reason: string };
  /** How the file source is made of files: `file`, `drop-ins` or `file + drop-ins`. */
  readonly shape?: string;
  /** Present and not a JSON object: an admin one stops Claude Code from starting. */
  readonly broken: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * What one source's read comes to. `label` names the source in a report (a path, `per-user managed
 * preferences`, `Registry: HKLM\SOFTWARE\Policies\ClaudeCode\Settings`).
 */
export const readAdminDocument = (
  source: ManagedSource,
  read: SourceRead,
  label?: string,
): AdminDocument => call('readAdminDocument', source, read, label);

/** What a failure to read a source costs, when no admin source supplies a policy (managed-settings.md, errors.md). */
export interface UnreadableVerdict {
  /** Claude Code exits at startup. */
  readonly exits: boolean;
  /** `/status` and `claude doctor` record it, and a `-p` run prints it to stderr. */
  readonly recorded: boolean;
  readonly message: string;
}

/** A cause that exits outranks one that starts the session without the source: the stricter outcome wins. */
export const harsherCause = (a: UnreadableCause, b: UnreadableCause): UnreadableCause =>
  call('harsherCause', a, b);

/** An issue about a failed read, an error where the cause exits and a warning where it does not. */
export const unreadableIssue = (
  cause: UnreadableCause,
  path: string,
  message: string,
): PluginIssue => call('unreadableIssue', cause, path, message);

/** What each cause comes to: the operating system's denial starts the session without the source; any other failure exits. */
export const unreadableVerdict = (
  cause: UnreadableCause,
  source: string,
  reason: string,
): UnreadableVerdict => call('unreadableVerdict', cause, source, reason);

/** The issue of a verdict, an error when it stops Claude Code. */
export const verdictIssue = (verdict: UnreadableVerdict, path: string): PluginIssue =>
  call('verdictIssue', verdict, path);
