/**
 * The files `claude plugin init` writes (plugins/create.md, "Scaffold the plugin with `claude plugin
 * init`" and "Share the plugin through a repository"; plugins/cli-reference.md, "plugin init"): a
 * pure plan, the host writes. The command "creates `~/.claude/skills/my-tool/` with a
 * `.claude-plugin/plugin.json` and a root `SKILL.md`"; `--with skills` also scaffolds a skill under
 * `skills/`. To share a plugin through one repository "create the same layout yourself at
 * `<project>/.claude/skills/<name>/`".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/plugin_scaffold.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('loading/plugin-scaffold');

/** Where the plugin folder goes: the personal skills directory, or a project's. */
export const SCAFFOLD_SCOPES: readonly ['user', 'project'] = call('SCAFFOLD_SCOPES');
export type ScaffoldScope = (typeof SCAFFOLD_SCOPES)[number];

export interface ScaffoldFile {
  /** Relative to the plugin folder. */
  readonly path: string;
  readonly content: string;
}

export interface ScaffoldPlan {
  /** The plugin folder: `~/.claude/skills/<name>` or `.claude/skills/<name>` (relative to the project). */
  readonly directory: string;
  readonly files: readonly ScaffoldFile[];
  /** `✔ Created plugin "<name>" at <directory>`. */
  readonly created: string;
  /** `It will auto-load next session as <name>@skills-dir. Run /reload-plugins to load it now.` */
  readonly loadsAs: string;
}

export interface ScaffoldInput {
  readonly name: string;
  readonly scope: ScaffoldScope;
  /** `--with skills`: also scaffold a skill under `skills/`. */
  readonly withSkills?: boolean;
}

/** The plan, or why the name cannot be a plugin's. */
export const planPluginScaffold = (
  input: ScaffoldInput,
): { readonly plan: ScaffoldPlan } | { readonly issues: readonly PluginIssue[] } =>
  call('planPluginScaffold', input);
