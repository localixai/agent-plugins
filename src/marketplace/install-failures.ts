import { coreModule } from '../wasm/bridge.ts';
import type { CommitFailure } from './ports.ts';

/**
 * The messages of an install that fails after the plugin was fetched (plugins/troubleshooting.md,
 * "Plugin archive integrity check failed", "An npm plugin source must name a registry package",
 * "This plugin uses a source type your Claude Code version does not support", "Plugin <name> has a
 * corrupt manifest file or has an invalid manifest file", "Could not move the new copy of this
 * plugin version into <path>", "does not load (...), so Claude Code ignores the whole file"). The
 * docs quote some of them whole and some in fragments; every fragment they quote is here verbatim,
 * and the words that join the fragments are the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_failures.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/install-failures');

/** The message of a `sha256` pin that does not match, as the docs print it whole. */
export const archiveIntegrityMessage = (url: string, expected: string, actual: string): string =>
  call('archiveIntegrityMessage', url, expected, actual);

/**
 * `"<value>" was not installed: <reason>. An npm plugin source must name a registry package ...`.
 * The docs' example reason is `it is not an http or https link`; `reason` is a clause that starts
 * with `it`.
 */
export const npmRefusalMessage = (value: string, reason: string): string =>
  call('npmRefusalMessage', value, reason);

/** The message of a source type this version cannot fetch, with the line the docs say follows it. */
export const UNSUPPORTED_SOURCE_TYPE: string = call('UNSUPPORTED_SOURCE_TYPE');

// ---- the manifest of a fetched plugin ----

/** `Failed to install plugin "<name>@<marketplace>": <message>`: the shell's prefix, with the plugin's real name. */
export const failedToInstallPlugin = (id: string, message: string): string =>
  call('failedToInstallPlugin', id, message);

/**
 * What is wrong with `.claude-plugin/plugin.json` of a plugin that was fetched: not JSON, or JSON
 * that fails the schema. `name` is what the message calls the plugin: "In the shell, the `<name>`
 * in this line can be a temporary directory name", so the caller passes what it has.
 */
export type ManifestProblem =
  | { readonly parse: string }
  | { readonly validation: readonly string[] };

export const manifestFailureMessage = (name: string, problem: ManifestProblem): string =>
  call('manifestFailureMessage', name, problem);

// ---- moving the new copy into the cache ----

/**
 * `Could not move the new copy of this plugin version into <path>: <why>`, then what became of the
 * copy installed before ("The message says what happened to the copy that was installed before"),
 * then the way out. A `Left on disk` list names the set-aside folders. An abort is not a failed
 * move, so it has no message.
 */
export const moveFailureMessage = (path: string, failure: CommitFailure): string | undefined =>
  call('moveFailureMessage', path, failure);

// ---- a settings file that does not load ----

/**
 * Why a settings file does not load: a value Claude Code does not accept (`keys` names the settings
 * in file order) or a top level that is not an object.
 */
export type IgnoredSettings =
  | { readonly invalid: readonly string[] }
  | { readonly notObject: true };

/**
 * The warning after the success line of `claude plugin install`, `enable`, `disable` and
 * `claude plugin marketplace add`: `⚠ <path> does not load (<why>), so Claude Code ignores the whole
 * file, ...`. With several bad values the text names the first and counts the others.
 */
export const settingsIgnoredWarning = (path: string, problem: IgnoredSettings): string =>
  call('settingsIgnoredWarning', path, problem);
