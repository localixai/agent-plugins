/**
 * Host release numbers, as the docs' "since v2.1.N" rules name them: one comparison for every such
 * rule. A leading `v`, surrounding blanks and text after the number (`2.1.320 (Claude Code)`) are
 * tolerated; a pre-release (`2.1.310-beta.1`) sorts before its release, its dot-separated parts
 * compared numerically when both are numbers, as in semver. A text with no leading
 * `major.minor.patch` is no version (judgment, spec/judgments.md). The order is node-semver's
 * precedence; plugin version ranges are `marketplace/semver`'s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/versions.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/versions');

/** Negative, zero or positive as `a` is before, the same as or after `b`; `undefined` when either is no version. */
export const compareVersions = (a: string, b: string): number | undefined =>
  call('compareVersions', a, b);

/** Whether `version` is `minimum` or later; a text that is no version is never at least anything. */
export const versionAtLeast = (version: string, minimum: string): boolean =>
  call('versionAtLeast', version, minimum);
