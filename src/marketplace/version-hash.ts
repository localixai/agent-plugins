/**
 * What a plugin version is hashed from (plugins/loading.md, "How Claude Code computes the version";
 * plugins/marketplace-reference.md, "Copy mode and link mode"): the files a `command` source copied,
 * the real path and top-level entries of a linked directory, and the subdirectory path of a
 * `git-subdir` source. The docs say what the hash covers, not how its input is laid out, so the
 * layouts below are judgments (spec/judgments.md). Every listing is sorted here by code unit, never
 * trusted from the port, so a digest never depends on the order a folder lists its entries in.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/version_hash.rs`, docs/RUST.md).
 */
import type { PluginFiles } from '../ports/files.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/version-hash');

/** "The commit SHA of the source, shortened to 12 characters": a hex digest cut to 12, lower case. */
export const shortHash12 = (hex: string): string => call('shortHash12', hex);

/**
 * The version of a `copy`-mode `command` source: "a hash of the copied files", 12 characters. The
 * input is one line per file or link in path order (`<path> NUL <digest of its bytes>`, or `<path>
 * NUL -> <link target>`), so the same files give the same hash ("a re-run that produces identical
 * files counts as up to date") and moving content between paths changes it.
 */
export const copyModeHash = (
  files: PluginFiles,
  sha256: Sha256,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<string> => call.async('copyModeHash', files, sha256, options);

/**
 * The version of a `link`-mode `command` source: "the printed directory's real path and its
 * top-level entries, not ... the files inside them". The input is the real path and the top-level
 * names in order; nothing is opened, so a link-mode plugin of any size has a version.
 */
export const linkModeHash = (
  realPath: string,
  files: PluginFiles,
  sha256: Sha256,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<string> => call.async('linkModeHash', realPath, files, sha256, options);

/**
 * The hash a `git-subdir` version carries ("a hash of the subdirectory path"): the first 8 hex
 * digits of the SHA-256 of the path, written without a leading `./` or a trailing `/` so the two
 * spellings of one directory give one version.
 */
export const subdirectoryPathHash = (path: string, sha256: Sha256): Promise<string> =>
  call.async('subdirectoryPathHash', path, sha256);
