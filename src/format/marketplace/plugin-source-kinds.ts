import { error, type PluginIssue } from '../issues.ts';
import { defined } from '../defined.ts';
import { archiveUrlIssues } from './archive-url.ts';
import {
  type Checked,
  fieldError,
  invalidInput,
  isText,
  optionalDigest,
  optionalText,
  readGitPin,
  refined,
  required,
  type RawSource,
  type SourceSite,
} from './plugin-source-fields.ts';
import type { PluginSource, PluginSourceKind } from './types.ts';

/**
 * The object plugin sources, one parser each (plugins/marketplace-reference.md, "Plugin sources"
 * and its sub-sections): the field rules the docs state. A parser returns the source, or the issues
 * that say why the object matches no type. What a field means at fetch time is the fetch plan's.
 */

export type ObjectKind = Exclude<PluginSourceKind, 'relative'>;
export type Parsed = PluginSource | readonly PluginIssue[];
export type ObjectParser = (raw: RawSource, site: SourceSite) => Parsed;

function github(raw: RawSource, site: SourceSite): Parsed {
  const repo = required(raw, 'repo');
  const pin = readGitPin(raw, site);
  if (repo === undefined || pin === undefined) return invalidInput(site);
  return refined({ kind: 'github', repo, ...defined(pin.value) }, pin.issues);
}

function url(raw: RawSource, site: SourceSite): Parsed {
  const address = required(raw, 'url');
  const pin = readGitPin(raw, site);
  if (address === undefined || pin === undefined) return invalidInput(site);
  return refined({ kind: 'url', url: address, ...defined(pin.value) }, pin.issues);
}

function gitSubdir(raw: RawSource, site: SourceSite): Parsed {
  const address = required(raw, 'url');
  const path = required(raw, 'path');
  const pin = readGitPin(raw, site);
  if (address === undefined || path === undefined || pin === undefined) return invalidInput(site);
  return refined({ kind: 'git-subdir', url: address, path, ...defined(pin.value) }, pin.issues);
}

/** "An `npm` `package` containing `..`" is `Invalid input` ("Invalid input on a source"). */
function npm(raw: RawSource, site: SourceSite): Parsed {
  const pkg = required(raw, 'package');
  const version = optionalText(raw, 'version');
  const registry = optionalText(raw, 'registry');
  if (pkg === undefined || version === undefined || registry === undefined) {
    return invalidInput(site);
  }
  if (pkg.includes('..')) return invalidInput(site);
  return {
    kind: 'npm',
    package: pkg,
    ...defined({ version: version.value, registry: registry.value }),
  };
}

function archive(raw: RawSource, site: SourceSite): Parsed {
  const address = required(raw, 'url');
  const sha256 = optionalDigest(raw, site);
  if (address === undefined || sha256 === undefined) return invalidInput(site);
  return refined({ kind: 'archive', url: address, ...defined({ sha256: sha256.value }) }, [
    ...archiveUrlIssues(address, site),
    ...sha256.issues,
  ]);
}

const MAX_COMMAND_LENGTH = 500;
const PRINTABLE_ASCII = /^[\x20-\x7E]*$/;
const FOUR_SPACES = / {4}/;
const COMMAND_TEXT =
  'command must be printable ASCII (letters, digits, punctuation, single spaces) with no runs of 4 or more spaces';
const COMMAND_LENGTH = 'command must not be longer than the install consent UI can display';

/**
 * The rules for a command string ("command plugin source": "Write it as printable ASCII, at most
 * 500 characters, with no run of four or more spaces"), with the two messages of spec §15.6.6 and
 * J15.8 (oracle 2.1.325): over 500 characters the length message; a character outside U+0020 to
 * U+007E, or a run of four spaces, the text message; both, length first, when both break.
 */
export function commandTextIssues(command: string, path: string): PluginIssue[] {
  const found: PluginIssue[] = [];
  if (command.length > MAX_COMMAND_LENGTH) found.push(error(path, COMMAND_LENGTH));
  if (!PRINTABLE_ASCII.test(command) || FOUR_SPACES.test(command)) {
    found.push(error(path, COMMAND_TEXT));
  }
  return found;
}

const DEFAULT_TIMEOUT_SECONDS = 60;
const MAX_TIMEOUT_SECONDS = 600;

function timeoutRangeIssues(value: number, site: SourceSite): PluginIssue[] {
  if (value <= 0) return [fieldError(site, 'timeout', 'Too small: expected number to be >0')];
  if (value <= MAX_TIMEOUT_SECONDS) return [];
  return [fieldError(site, 'timeout', `Too big: expected number to be <=${MAX_TIMEOUT_SECONDS}`)];
}

/** `timeout`: a whole number of seconds from 1 to 600, 60 when absent; not a whole number is no value at all. */
function readTimeout(raw: RawSource, site: SourceSite): Checked<number> | undefined {
  const value = raw.timeout;
  if (value === undefined) return { value: DEFAULT_TIMEOUT_SECONDS, issues: [] };
  if (typeof value !== 'number' || !Number.isInteger(value)) return undefined;
  return { value, issues: timeoutRangeIssues(value, site) };
}

/** `mode`: `copy` (the default) or `link` ("Copy mode and link mode"). */
const readMode = (value: unknown): 'copy' | 'link' | undefined => {
  if (value === undefined) return 'copy';
  return value === 'copy' || value === 'link' ? value : undefined;
};

function readCommand(raw: RawSource, site: SourceSite): Parsed {
  const text = raw.command;
  const timeout = readTimeout(raw, site);
  const mode = readMode(raw.mode);
  if (!isText(text) || timeout === undefined || mode === undefined) {
    return invalidInput(site);
  }
  const broken = [...commandTextIssues(text, `${site.path}.command`), ...timeout.issues];
  return refined({ kind: 'command', command: text, timeoutSeconds: timeout.value, mode }, broken);
}

/** One parser per object kind: a new `PluginSourceKind` fails the typecheck until it has one. */
export const OBJECT_PARSERS: { readonly [K in ObjectKind]: ObjectParser } = {
  github,
  url,
  'git-subdir': gitSubdir,
  npm,
  archive,
  command: readCommand,
};
