/**
 * Where a skill or command file of a scope lives.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/scan/file_location.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { ScanScope } from './locations.ts';

const call = coreModule('skills/scan/file-location');

/** A skill folder (`SKILL.md`) or an older command file (skills.md, "Command files"). */
export type ScopeFileKind = 'skill' | 'command';

/**
 * Where a skill or command file named `name` of `scope` lives, below `scopeFolder`: the scope's
 * `.claude` folder (for `user`, the configuration directory). The rule the scan reads by
 * (`SCOPE_RULES`; skills.md, "Choose where skills load"): `skills/<name>/SKILL.md` and
 * `commands/<name>.md`. `undefined` when the scope reads no file of that kind: the managed, nested
 * and synced scopes read no commands (a synced skill's folder is claude.ai's: `skills/synced/`).
 */
export const scopeFilePath = (
  scope: ScanScope,
  kind: ScopeFileKind,
  scopeFolder: string,
  name: string,
): string | undefined => call('scopeFilePath', scope, kind, scopeFolder, name);
