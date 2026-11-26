import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The form of a plugin root, decided before any component is read (Agent Plugins Extended §1.3):
 *
 * | `.claude-plugin/plugin.json` | root `plugin.json`            | form                          |
 * | ---------------------------- | ----------------------------- | ----------------------------- |
 * | present                      | absent, or no core `$schema`  | Claude Code plugin            |
 * | present                      | with a core `$schema`         | Claude Code plugin, dual      |
 * | absent                       | with a core `$schema`         | core package                  |
 * | absent                       | absent, or no core `$schema`  | Claude Code plugin, no manifest |
 *
 * A root `plugin.json` whose `$schema` is no canonical core identifier, that is not JSON or not an
 * object, is an ordinary file (J1.1). A dual package loads as a Claude Code plugin only (§1.3.1,
 * J1.2); `dual` says that it also carries a core manifest, which a host shows in its details.
 *
 * Judgment (spec/judgments.md): a root `plugin.json` that is a link leading outside the root is not
 * read, so it is not a core manifest: the root is then a Claude Code plugin, which Claude Code would
 * load the same way since it never reads that file.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/form.rs`, docs/RUST.md).
 */
const call = coreModule('loading/core/form');

export const PLUGIN_FORMS: readonly ['claude-code', 'core'] = call('PLUGIN_FORMS');
export type PluginForm = (typeof PLUGIN_FORMS)[number];

export interface PluginFormDecision {
  readonly form: PluginForm;
  /** The root carries both manifests (a dual package); its form is then `claude-code`. */
  readonly dual: boolean;
}

/** Decides the form of the plugin root `files` reads (§1.3). */
export const detectPluginForm = (input: {
  readonly files: PluginFiles;
  readonly signal?: AbortSignal | undefined;
}): Promise<PluginFormDecision> => call.async('detectPluginForm', input);
