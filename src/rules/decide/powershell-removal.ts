import { coreModule } from '../../wasm/bridge.ts';
import type { PathContext } from '../types.ts';

/**
 * permission-modes.md, "Remove-Item in PowerShell": the two cases Claude Code "denies ... in every
 * mode, without asking you" (`bypassPermissions` included: "The Remove-Item in PowerShell denies
 * also apply in this mode"):
 * - `Remove-Item` on a system path ("the filesystem root and its top-level directories, drive roots
 *   and their top-level directories, and your home directory"), or on a wildcard ("a bare `*`, or
 *   any target ending in `/*` or `\*`, including a glob under a shell variable such as `$dir/*`");
 * - `rd`, `rmdir`, `del` and `erase` run through `cmd` on a system path, where "a PowerShell
 *   variable that follows literal text" counts as empty and "a trailing wildcard counts as the folder
 *   it empties". `CLAUDE_CODE_DISABLE_POWERSHELL_CMD_RM_DENY=1` turns this case off
 *   (`DecideContext.powershellCmdRemovalDenyOff`, the host's reading of the launch environment).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/powershell_removal.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/decide/powershell-removal');

/** Whether a PowerShell command holds a removal the docs deny in every mode. */
export const powershellRemovalDenied = (
  command: string,
  paths: PathContext,
  cmdCheckOff: boolean,
): boolean => call('powershellRemovalDenied', command, paths, cmdCheckOff);
