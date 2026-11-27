import type { BodyReader, InvocableHead } from '../../format/skill-types.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ScanScope } from './locations.ts';
import type { ScannedEntry } from './scan-types.ts';

/**
 * Which skill a name runs when several share it (skills.md, "Resolve skills that share a name", "How
 * a skill gets its command name", "Run a same-named skill from an added directory", "When a synced
 * skill name matches another command"). The scans of every scope go in, the skills and commands a
 * session lists come out, each with the names that invoke it.
 *
 * Every scope has a naming strategy in one registry typed over the scopes: the names it prefers, the
 * names it falls back to when none of those is free, and whether names are compared as the synced
 * skills' are (ignoring case, spacing and invisible characters).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/resolve.rs`, docs/RUST.md).
 */

const call = coreModule('skills/scan/resolve');

/** Who holds a name: a skill or command file of the folders, or (absent) another source of commands. */
export interface NameHolder {
  readonly scope: ScanScope;
  readonly path: string | undefined;
}

/** A name an entry would have liked and did not get, and who has it (skills.md, "When a synced skill name matches another command"). */
export interface YieldedName {
  readonly name: string;
  /** The folder skill or command file that uses it; absent for a built-in command, a plugin skill or an MCP prompt. */
  readonly heldBy: NameHolder | undefined;
}

export interface ResolvedFolderSkill {
  /** The skill or command, without its body, with its listed command (`/apps/web:deploy`, `/cli:verify`). */
  readonly invocable: InvocableHead;
  /** Reads its body when it runs (`ScannedEntry.readBody`). */
  readonly readBody: BodyReader;
  readonly scope: ScanScope;
  /** Every name that invokes it, the listed command first. */
  readonly names: readonly string[];
  /** The names it prefers that another command already used: a synced skill that lost its short name lists it here. */
  readonly yielded: readonly YieldedName[];
}

/** An entry that lost its name to another, for a host that wants to say so. */
export interface ShadowedEntry {
  readonly scope: ScanScope;
  readonly command: string;
  readonly path: string | undefined;
}

export interface Resolution {
  readonly resolved: readonly ResolvedFolderSkill[];
  readonly shadowed: readonly ShadowedEntry[];
}

export interface ResolveFolderOptions {
  /** The working directory the nested skills' qualified names are relative to. */
  readonly cwd: string;
  /** The added directories, in order: their folder names prefix a clashing skill. */
  readonly addedDirectories?: readonly string[];
  /**
   * Commands other sources already use: built-in commands, bundled skills, plugin skills, MCP
   * prompts. Only a synced skill's short name yields to them.
   */
  readonly taken?: Iterable<string>;
}

export const resolveFolderSkills = (
  scanned: readonly ScannedEntry[],
  options: ResolveFolderOptions,
): Resolution => call('resolveFolderSkills', scanned, options);
