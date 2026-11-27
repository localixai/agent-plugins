/**
 * What a string a shell parses needs in its process environment for the references
 * `SubstitutionScope.shellPaths` left in it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/shell_exports.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { SubstitutionScope } from './scope.ts';
import type { TokenReference } from './substitute.ts';
import type { SubstitutionSurface } from './surfaces.ts';
import type { PluginVariable } from './variables.ts';

const call = coreModule('loading/resolve/shell-exports');

type Ask = (name: PluginVariable) => Promise<string>;

/** What the exports read from the scope: its `shellPaths` and the paths another layer owns. */
type ExportScope = Pick<SubstitutionScope, 'shellPaths' | 'extra' | 'extraPaths'>;

/**
 * What a string a shell parses needs in its process environment for the references
 * `SubstitutionScope.shellPaths` left in it: the path variables it references (and the `extra`
 * paths, `CLAUDE_SKILL_DIR`), nothing for pasted paths. The one rule for a monitor `command`, an
 * MCP `headersHelper` and a skill's `!` command, whose processes get no path variables of their own
 * (a hook's environment already carries all three). Pass the substituter's `variable` so the data
 * folder is asked for once.
 */
export const shellExports = (
  surface: SubstitutionSurface,
  referenced: readonly TokenReference[],
  scope: ExportScope,
  ask: Ask,
): Promise<Record<string, string>> => call.async('shellExports', surface, referenced, scope, ask);
