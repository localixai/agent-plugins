/**
 * The range side of `semver` (node-semver; `format/versions.ts` orders releases with it), the range grammar plugins/dependencies.md names
 * for a dependency's `version` ("a semantic-version range such as `~2.1.0`, `^2.0`, `>=1.4`, or
 * `=2.1.0`", linking github.com/npm/node-semver#ranges). Every rule below is node-semver's own, with
 * its default options: a range never matches a pre-release version unless it opts in with a
 * pre-release suffix on a comparator ("`^2.0.0-0`").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/semver.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/semver');

/** Whether `range` is a valid node-semver range. */
export const isValidRange = (range: string): boolean => call('isValidRange', range);

/** A strict version (`2.1.0`, `2.0.0-beta.1`), as a git tag carries it: no `v`, `=` or blanks. */
export const strictVersion = (text: string): string | undefined => call('strictVersion', text);

/** The version `text` names, spelled one way (`v2.1.100` is `2.1.100`), or `undefined` when it is no version. */
export const normalizedVersion = (text: string): string | undefined =>
  call('normalizedVersion', text);

/** Whether `version` satisfies `range`; a version that is not semver satisfies nothing. */
export const satisfiesRange = (version: string, range: string): boolean =>
  call('satisfiesRange', version, range);

/** The highest of `versions` inside `range`, or `undefined`. */
export const highestSatisfying = (versions: readonly string[], range: string): string | undefined =>
  call('highestSatisfying', versions, range);

/** The highest of `versions` by semver order, or `undefined` when there are none. */
export const highestVersion = (versions: readonly string[]): string | undefined =>
  call('highestVersion', versions);

/**
 * Whether some version satisfies all of `ranges` (each valid), or `undefined` when there are too
 * many `||` alternatives to decide ("has version requirements too complex to intersect"). The
 * alternatives are distributed (`(a || b) && c` is `a && c || b && c`); the combination is empty
 * unless the chosen intervals meet pairwise, which for intervals means a common point (Helly).
 */
export const rangesOverlap = (ranges: readonly string[]): boolean | undefined =>
  call('rangesOverlap', ranges);
