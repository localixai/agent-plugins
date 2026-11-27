/**
 * Reads the skills and command files of the scope folders through the `FolderScan` port (skills.md,
 * "Choose where skills load", "How a skill gets its command name"): `<base>/.claude/skills/<name>/SKILL.md`
 * and the flat `<base>/.claude/commands/**.md`, then the same below each extra scope folder the host
 * names (`ScopeContext.extraScopeFolders`). Nothing is resolved here: two skills of one name are
 * both returned, with the scope each came from, for `resolveFolderSkills` to decide. Entries carry no
 * body (`ScannedEntry.readBody` reads it when the skill runs).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/scan.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ScanContext, ScanScope } from './locations.ts';
import type { ScanOutcome } from './scan-types.ts';

export type { ScanOutcome, ScannedEntry, SkillsDirectoryPlugin } from './scan-types.ts';

const call = coreModule('skills/scan/scan');

/**
 * One scope: every base directory of it, in order, below `.claude` and then below each of the host's
 * extra folders (`scopeFoldersOf`), so an extra folder's entries come after every `.claude` entry of
 * the scope and lose a shared name to them.
 */
export const scanScope = (context: ScanContext, scope: ScanScope): Promise<ScanOutcome> =>
  call.async('scanScope', context, scope);

/**
 * Every scope of a session, in precedence order: managed, personal, project (the working directory
 * and its parents), the added directories, and the skills synced from claude.ai. The skills below
 * the working directory are `scanNested`'s, as the session touches files there.
 */
export const scanFolders = (context: ScanContext): Promise<ScanOutcome> =>
  call.async('scanFolders', context);
