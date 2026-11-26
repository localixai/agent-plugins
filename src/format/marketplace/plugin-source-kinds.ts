/**
 * The object plugin sources, one parser each (plugins/marketplace-reference.md, "Plugin sources"
 * and its sub-sections): the field rules the docs state. A parser returns the source, or the issues
 * that say why the object matches no type. What a field means at fetch time is the fetch plan's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/plugin_source_kinds.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { RawSource, SourceSite } from './plugin-source-fields.ts';
import type { PluginSource, PluginSourceKind } from './types.ts';

const call = coreModule('format/marketplace/plugin-source-kinds');

export type ObjectKind = Exclude<PluginSourceKind, 'relative'>;
export type Parsed = PluginSource | readonly PluginIssue[];
export type ObjectParser = (raw: RawSource, site: SourceSite) => Parsed;

/**
 * The rules for a command string ("command plugin source": "Write it as printable ASCII, at most
 * 500 characters, with no run of four or more spaces"), with the two messages of spec §15.6.6 and
 * J15.8 (oracle 2.1.325): over 500 characters the length message; a character outside U+0020 to
 * U+007E, or a run of four spaces, the text message; both, length first, when both break.
 */
export const commandTextIssues = (command: string, path: string): PluginIssue[] =>
  call('commandTextIssues', command, path);

/** One parser per object kind: a new `PluginSourceKind` fails the typecheck until it has one. */
export const OBJECT_PARSERS: { readonly [K in ObjectKind]: ObjectParser } = call('OBJECT_PARSERS');
