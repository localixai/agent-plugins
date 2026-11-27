import type { AgentRecord } from '../format/agent-record.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { OutputStyle } from '../format/output-style.ts';
import type { Command, Skill } from '../format/skill-types.ts';
import type { Plugin } from '../loading/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import { givenOf, markdownOf } from './core-view.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/host/catalog.rs`, docs/RUST.md).
 */

/** What one plugin's Markdown components read to: the formats' own readers, once, at load. */
export interface PluginCatalog {
  readonly skills: readonly Skill[];
  readonly commands: readonly Command[];
  readonly agents: readonly AgentRecord[];
  readonly outputStyles: readonly OutputStyle[];
  readonly issues: readonly PluginIssue[];
}

const call = coreModule('host/catalog');

/**
 * Reads the skills, commands, agents and output styles of a loaded plugin with the format readers.
 * Judgment (spec/judgments.md): the catalog is cached by the identity of the `Plugin` object, which
 * is immutable, so a host update that keeps a plugin does not read its Markdown again; a new
 * object, even for the same id, is read.
 */
export function readCatalog(plugin: Plugin): PluginCatalog {
  const given = givenOf(plugin);
  given.catalog ??= call(
    'readCatalog',
    given.handle,
    given.core === undefined ? markdownOf(plugin) : undefined,
  );
  return given.catalog;
}
