/**
 * One skills container that is not a scope's `.claude/skills`: the folder a plugin's manifest lists
 * under `skills` (plugins/components.md, "Skills"), a store's or a curator's skills directory. Its
 * immediate children are the skill folders, read as `scanFolders` reads them, and the entries carry
 * no body.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/container.rs`, docs/RUST.md).
 */
import type { SkillOrigin } from '../../format/skill-types.ts';
import type { FolderScan } from '../../ports/folder-scan.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ScanScope } from './locations.ts';
import type { ScanOutcome } from './scan-types.ts';

const call = coreModule('skills/scan/container');

export interface ContainerRequest {
  /** What the entries report as their scope. */
  readonly scope: ScanScope;
  readonly origin: SkillOrigin;
  /**
   * The container holds a `SKILL.md` itself: the folder is one skill and its own subfolders are its
   * files, not more skills ("skills": ["./"] lists a plugin whose root is one skill). It goes by this
   * name unless its frontmatter names it.
   */
  readonly rootName: string;
  readonly signal?: AbortSignal;
}

/**
 * The skills of `container` (a path relative to `dir`, `.` for `dir` itself) read through the port.
 * A folder that does not exist has no skills.
 */
export const scanSkillContainer = (
  folders: FolderScan,
  dir: string,
  container: string,
  request: ContainerRequest,
): Promise<ScanOutcome> => call.async('scanSkillContainer', folders, dir, container, request);
