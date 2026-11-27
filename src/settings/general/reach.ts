/** Implemented by the Rust core (`crates/agent-plugins/src/settings/general/reach.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/reach');

/**
 * Whose sessions a settings file reaches (settings.md, "Settings files and who they affect" and
 * "Compare the scope of each settings file"). The file kinds are the four a person can name; `flag`
 * and the rest are not files of a machine.
 */
export const FILE_SCOPES: readonly ['user', 'project', 'local', 'managed'] = call('FILE_SCOPES');

export type FileScope = (typeof FILE_SCOPES)[number];

/** Whether a scope a host holds as a string is one of the four file scopes. */
export const isFileScope = (scope: string): scope is FileScope => call('isFileScope', scope);

/** What a host tells about the file besides its scope. */
export interface ReachFacts {
  /** The project file is committed to version control (a teammate's clone and a cloud session then get it). */
  readonly committed: boolean;
  /** The managed settings come from the claude.ai console (server-managed), not a file or an MDM policy. */
  readonly serverManaged: boolean;
}

export interface Reach {
  /** On your own machine: every project, or the one project the file sits in. */
  readonly yourMachine: 'every project' | 'this project';
  /** A teammate's machine, with their own clone of the project. */
  readonly teammate: boolean;
  /** A cloud session on the project. */
  readonly cloudSession: boolean;
}

/** Where a setting saved in a file of this scope applies. */
export const reachOf = (scope: FileScope, facts: ReachFacts): Reach =>
  call('reachOf', scope, facts);
