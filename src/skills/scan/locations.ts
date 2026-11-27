/**
 * Where skills and command files are read from, one rule per scope (skills.md, "Choose where skills
 * load"; permissions.md, "Additional directories grant file access, not configuration"). A registry
 * typed over the scopes, so a new scope fails the typecheck until it says where it reads and what it
 * loads as. `rank` is the precedence of "Resolve skills that share a name": the managed (enterprise)
 * directory over the personal folder over the project, then the skills below the working
 * directory, then the added directories, then the skills synced from claude.ai.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/locations.rs`, docs/RUST.md).
 * A rule crosses as its data: where each scope's folders are and which may be a plugin are the
 * core's, which scans them.
 */
import type { SkillOrigin } from '../../format/skill-types.ts';
import type { ScopeContext, ScopeRequest } from '../../foundation/scope-folders.ts';
import type { SkillsOrigin } from '../../settings/customization/sources.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('skills/scan/locations');

export type ScanScope = 'managed' | 'user' | 'project' | 'nested' | 'additional' | 'synced';

export const SCAN_SCOPES: readonly ScanScope[] = call('SCAN_SCOPES');

/** What a scan is asked for. */
export interface ScanRequest extends ScopeRequest {
  /** Directories below `cwd` whose `.claude/skills` the session loaded (see `scanNested`). */
  readonly nestedDirectories?: readonly string[];
}

export interface ScanContext extends ScopeContext<ScanRequest> {
  /** Whether `strictPluginOnlyCustomization` lets a source load; absent, every source does. */
  readonly allows?: (origin: SkillsOrigin) => boolean;
}

interface Container {
  /** The folder below a base directory's scope folder (`.claude`, or an extra one): `skills`. */
  readonly folder: string;
  /** The name `strictPluginOnlyCustomization` knows this source by. */
  readonly origin: SkillsOrigin;
}

export interface ScopeRule {
  readonly rank: number;
  /** What a skill or command of this scope is read as. */
  readonly origin: SkillOrigin;
  readonly skills: Container | undefined;
  readonly commands: Container | undefined;
  /**
   * Whether the session watches the skills folder for changes (skills.md, "Edit a skill during a
   * session": the personal folder, the project's and each added directory's).
   */
  readonly watched: boolean;
  /** Whether the host's extra scope folders are read too (`scopeFoldersOf`), or `.claude` alone. */
  readonly extraFolders: boolean;
}

export const SCOPE_RULES: { readonly [S in ScanScope]: ScopeRule } = call('SCOPE_RULES');
