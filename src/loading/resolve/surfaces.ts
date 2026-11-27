/**
 * Where a string is substituted, finer than a component where the docs split one: a hook runs in
 * exec form or shell form (or through PowerShell), and an MCP server is stdio or remote, whose
 * `headersHelper` runs through a shell.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/surfaces.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { VariableComponent } from './field-map.ts';
import type { FieldPolicy, SubstitutionScope } from './scope.ts';
import type { Substituted } from './substitute.ts';

const call = coreModule('loading/resolve/surfaces');

export const SUBSTITUTION_SURFACES: readonly [
  'hook-exec',
  'hook-shell',
  'hook-powershell',
  'monitor-command',
  'mcp-stdio',
  'mcp-remote',
  'mcp-headers-helper',
  'lsp',
  'markdown-content',
  'skill-bash',
  'skill-powershell',
] = call('SUBSTITUTION_SURFACES');
export type SubstitutionSurface = (typeof SUBSTITUTION_SURFACES)[number];

/** The component whose row of the table each surface belongs to. */
export const SURFACE_COMPONENT: { readonly [S in SubstitutionSurface]: VariableComponent } =
  call('SURFACE_COMPONENT');

/**
 * The differences between surfaces, from the docs:
 * - `${user_config.*}`: exec-form `args`, MCP config, LSP config (manifest-reference, "Reference a
 *   saved value"); never shell-form hooks, monitors or `headersHelper` ("Fields that run through a
 *   shell"); non-sensitive only in skill and agent content.
 * - `${VAR}`: MCP `command`, `args`, `env`, `url`, `headers` (mcp.md, not `headersHelper`) and
 *   monitor `command` (components.md, "Monitors"). Judgment: nowhere else; a hook or LSP command
 *   keeps `${HOME}` for the process, and the docs give the kit no expansion to do.
 * - Windows paths: forward slashes, except exec-form hooks (native) and PowerShell hooks, where
 *   the placeholders become `${env:NAME}` (hooks.md, "Reference scripts by path in PowerShell hooks").
 *   A bash shell-form hook keeps `${NAME}` for the shell to read from the hook's environment, as
 *   Claude Code does (oracle 2.1.322), whatever the scope's `shellPaths`.
 * - `mcp-remote`: credential variables read as empty in `url` and `headers` (mcp.md, "Credential
 *   variables that read as empty"), `readsAsEmptyInRemoteMcp`.
 * - A hook with no `shell` field runs in PowerShell on Windows without Git Bash (hooks.md, "Exec
 *   form and shell form"); the kit cannot tell, so the caller picks `hook-shell` or `hook-powershell`.
 * - A skill's `!` command is skill content (skills.md, "Available string substitutions") that the
 *   Bash or the PowerShell tool runs ("How injected commands run"): `skill-bash` and
 *   `skill-powershell`, the text the shell receives; the permission check reads the same command
 *   as `markdown-content`.
 */
export const SURFACE_POLICY: { readonly [S in SubstitutionSurface]: FieldPolicy } =
  call('SURFACE_POLICY');

/**
 * The policy of a surface in this scope: `SURFACE_POLICY`, with a shell-parsed surface's paths as
 * the scope's `shellPaths` make them. Every resolver asks this, so the choice is made once.
 */
export const surfacePolicy = (
  surface: SubstitutionSurface,
  scope: Pick<SubstitutionScope, 'shellPaths'>,
): FieldPolicy => call('surfacePolicy', surface, scope);

/** `substituteTemplate` with the policy of a surface in this scope. */
export const substituteSurface = (
  surface: SubstitutionSurface,
  template: string,
  scope: SubstitutionScope,
  path = '',
): Promise<Substituted> => call.async('substituteSurface', surface, template, scope, path);
