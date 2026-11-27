import type { PortFailure } from '../../ports/failure.ts';
import type { PathResolver } from '../../ports/path-resolver.ts';
import type { PathContext, PermissionRule, PreparedCall, ToolCall } from '../types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * Symlinks (permissions.md, "Symlinks"): "the permission check covers two paths: the one Claude
 * requested and the file it resolves to". Matching reads `PreparedCall.resolvedPaths`; this file
 * fills it through the host's `PathResolver`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/path/symlink.rs`, docs/RUST.md).
 */
const call = coreModule('rules/path/symlink');

/**
 * A call with the real location of its path, ready for the path rules. A call whose path is not a
 * link comes back as it was; one with no path is untouched and the resolver is not asked.
 */
export const prepareCall = (
  toolCall: ToolCall,
  paths: PathContext,
  resolver: PathResolver,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<PreparedCall | PortFailure> =>
  call.async('prepareCall', toolCall, paths, resolver, options);

/**
 * The real location of every directory a `//`, `~/`, `/` or `./` deny or ask rule is written
 * through, and of every working directory: "On macOS and Linux, a deny or ask rule written through
 * a symlinked directory ... also applies at the directory's real location. For example, on macOS,
 * where `/etc` resolves to `/private/etc`, `Read(//etc/**)` blocks `/private/etc/hosts` too." The
 * anchor counts as such a directory, so a `./private/**` rule under a current directory reached
 * through a link (`/var/proj` is `/private/var/proj`) covers the real spelling too, and a working
 * directory reached through a link holds the files that resolve below its real location (judgments,
 * spec/judgments.md). Only the directories that are links come back. A host puts the map in
 * `PathContext.realDirectories`. Windows has none ("On macOS and Linux").
 */
export const resolveRuleDirectories = (
  rules: readonly PermissionRule[],
  paths: PathContext,
  resolver: PathResolver,
  options: { readonly signal?: AbortSignal | undefined } = {},
): Promise<ReadonlyMap<string, string>> =>
  call.async('resolveRuleDirectories', rules, paths, resolver, options);
