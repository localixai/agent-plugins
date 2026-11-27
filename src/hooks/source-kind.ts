/**
 * What a kind of declaration site decides about its handlers, wherever the hooks read it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/source_kind.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { HookSource, HookSourceKind } from './contracts.ts';

export interface SourcePolicy {
  /** A plugin's hooks file. */
  readonly plugin: boolean;
  /**
   * hooks.md, "Hook handler fields": "If you define the same handler in more than one settings
   * file, it runs once. A plugin's or skill's copy of the same handler stays separate."
   */
  readonly mergesCopies: boolean;
  /**
   * hooks.md, "Common fields": `once` is "Only honored for hooks declared in skill frontmatter;
   * ignored in settings files and agent frontmatter" (and so in a plugin's hooks file).
   */
  readonly honorsOnce: boolean;
  /**
   * `SessionEnd`: "your settings files" raise the budget, "plugin-provided hooks" do not (the
   * other kinds: judgment, spec/judgments.md).
   */
  readonly raisesSessionEndBudget: boolean;
  /**
   * A session registered it after it opened (a skill's, an agent's, the host's run): an update
   * keeps it, while the plugin and settings handlers are rebuilt from the new state.
   */
  readonly scoped: boolean;
}

const call = coreModule('hooks/source-kind');

/** One row per kind: a registry over the union, so a new kind is a compile error here. */
export const SOURCE_POLICY: { readonly [K in HookSourceKind]: SourcePolicy } =
  call('SOURCE_POLICY');

export const isPluginSource = (source: HookSource): boolean => call('isPluginSource', source);

/**
 * The name a plugin hook's notices give its plugin: the manifest name, else the plugin id. One
 * rule for the block error ("This hook comes from the <plugin> plugin.") and the missing
 * directory notice.
 */
export const pluginLabel = (entry: {
  readonly pluginId?: string | undefined;
  readonly source: HookSource;
}): string | undefined => call('pluginLabel', entry);
