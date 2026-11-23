import { rcompare, satisfies, valid, validRange } from 'semver';
import { isRecord } from '../protocol/is-record.ts';
import { note, type PluginNote } from './plugin-notes.ts';
import { errorAt, warningAt, type Finding } from './validate/finding.ts';
import { kindOf } from './validate/received.ts';

/**
 * `engines` of `plugin.json` and of a marketplace entry (spec §2.4, §15.4, Judgment J2.10): the
 * Claude Code versions a plugin works on, a node-semver range under `claude-code`. A value that is
 * not a range, or an `engines` that is not an object, is ignored at load; other keys are ignored.
 */

/**
 * What loading takes from an `engines` value: the range, or the `claude-code` value that is not
 * one (`invalid`), or neither.
 */
export interface EnginesRead {
  readonly range?: string;
  readonly invalid?: { readonly value: unknown };
}

export function readEngines(value: unknown): EnginesRead {
  if (!isRecord(value) || !Object.hasOwn(value, 'claude-code')) return {};
  const range = value['claude-code'];
  return typeof range === 'string' && validRange(range) !== null
    ? { range }
    : { invalid: { value: range } };
}

const shown = (value: unknown): string =>
  typeof value === 'string' ? `"${value}"` : kindOf(value);

/** The validator's findings for an `engines` value, at `<prefix>` and `<prefix>.claude-code`. */
export function enginesFindings(value: unknown, prefix = 'engines'): Finding[] {
  if (!isRecord(value)) {
    return [
      warningAt(
        prefix,
        `'engines' must be an object such as { "claude-code": ">=2.1.0" }; got ${kindOf(value)}. It will be ignored at load time.`,
      ),
    ];
  }
  const { invalid } = readEngines(value);
  if (invalid === undefined) return [];
  return [
    errorAt(
      `${prefix}.claude-code`,
      `Must be a version range such as ">=2.1.0"; got ${shown(invalid.value)}. It will be ignored at load time.`,
    ),
  ];
}

/**
 * Whether the running client may install, update or load the plugin (§2.4): `outside` is the range
 * that excludes it, absent when it fits; `exempt` says it loads anyway from a local folder.
 */
export interface EngineVerdict {
  readonly outside?: string;
  readonly exempt: boolean;
}

export interface EngineFacts {
  /** The running client's version, such as `2.1.330`. */
  readonly clientVersion: string;
  /** `--plugin-dir`, or a plugin loaded in place from a marketplace added from a local path. */
  readonly localFolder: boolean;
}

/** A valid range that excludes the client refuses the plugin, except from a local folder. */
export function engineVerdict(read: EnginesRead, facts: EngineFacts): EngineVerdict {
  const { range } = read;
  if (range === undefined || satisfies(facts.clientVersion, range, { includePrerelease: true })) {
    return { exempt: false };
  }
  return { outside: range, exempt: facts.localFolder };
}

/** True when the plugin must not install, update or load. */
export const engineRefuses = (verdict: EngineVerdict): boolean =>
  verdict.outside !== undefined && !verdict.exempt;

/**
 * The `engine-range-exempt` and `engine-range-invalid` notes a plugin loads with; without the
 * client's facts only the second can be known.
 */
export function engineNotes(read: EnginesRead, facts: EngineFacts | undefined): PluginNote[] {
  if (read.invalid !== undefined) {
    return [
      note(
        'engine-range-invalid',
        `Its Claude Code range ${shown(read.invalid.value)} isn't a version range, so it's ignored`,
      ),
    ];
  }
  if (facts === undefined) return [];
  const { outside, exempt } = engineVerdict(read, facts);
  return outside !== undefined && exempt
    ? [
        note(
          'engine-range-exempt',
          `It needs Claude Code ${outside}, this is ${facts.clientVersion}. It loads anyway, because it's read from a local folder`,
        ),
      ]
    : [];
}

/**
 * Why a plugin outside its range does not install, update or load. The spec quotes no message
 * for the refusal; this wording is the kit's (spec/judgments.md, J2.10).
 */
export const engineRefusal = (name: string, range: string, clientVersion: string): string =>
  `"${name}" needs Claude Code ${range}, this is ${clientVersion}`;

/** What the install success line continues with for a local-folder plugin outside its range (§17.4). */
export const engineInstallSuffix = (name: string, range: string, clientVersion: string): string =>
  `. Note: "${name}" needs Claude Code ${range}, this is ${clientVersion}, but loads anyway because it's read from a local folder`;

/** A version an install or update could take, with the `engines` it declares. */
export interface EngineCandidate {
  readonly version: string;
  readonly engines?: unknown;
}

/**
 * "An install or update picks the newest version whose range fits" (spec §2.4): the highest semver
 * among the candidates the client may take; `undefined` when none fits. A candidate whose version is
 * not semver is not compared (the kit's reading, spec/judgments.md, J2.10). Which versions exist, and
 * their `engines`, is what the host fetched (`<plugin>--v<version>` tags, §16.23).
 */
export function newestFittingVersion(
  candidates: readonly EngineCandidate[],
  facts: EngineFacts,
): string | undefined {
  return candidates
    .filter((one) => valid(one.version) !== null)
    .filter((one) => !engineRefuses(engineVerdict(readEngines(one.engines), facts)))
    .map((one) => one.version)
    .toSorted(rcompare)[0];
}
