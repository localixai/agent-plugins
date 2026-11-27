/**
 * How one component field of an entry joins the same field of plugin.json under strict mode
 * (plugins/manifest-reference.md, "How entry fields combine with `plugin.json`"): paths are
 * appended, a `commands` map adds its commands, and for `hooks` the entry's matchers replace the
 * manifest's per event.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/entry_append.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { PluginManifest } from '../manifest-schema.ts';

const call = coreModule('format/marketplace/entry-append');

type Paths = string | string[];
type Commands = PluginManifest['commands'];
type Hooks = PluginManifest['hooks'];

/** The manifest's paths followed by the entry's; when only one holds the field it stays as written. */
export const appendPaths = (
  manifest: Paths | undefined,
  entry: Paths | undefined,
): Paths | undefined => call('appendPaths', manifest, entry);

/**
 * `commands` takes paths or a map of command name to source or content. Two lists append; two maps
 * merge, the entry's command replacing a manifest command of the same name (spec/judgments.md); a
 * list and a map cannot be held in one field, so the manifest's stays and the entry's is reported.
 */
export const combineCommands = (
  manifest: Commands | undefined,
  entry: Commands | undefined,
): { value: Commands | undefined; issues: PluginIssue[] } =>
  call('combineCommands', manifest, entry);

/**
 * The hooks of a plugin once the entry's are laid over them: for an event both declare, the
 * entry's matchers replace the manifest's; an event only one declares keeps its own. Hook files the
 * manifest names cannot be read here, so they stay and the entry's object follows them (the caller
 * reports which events it replaces).
 */
export const mergeHooks = (
  manifest: Hooks | undefined,
  entry: Readonly<Record<string, unknown>>,
): Hooks => call('mergeHooks', manifest, entry);
