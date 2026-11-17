import { isRecord } from '../../protocol/is-record.ts';
import { canonicalJson } from '../canonical-json.ts';
import { errorIn, warningIn, type ValidationIssue } from '../validate/finding.ts';
import { readCoreManifest } from './manifest.ts';

/**
 * The manifest half of a dual package (Agent Plugins Extended §18.3.1): `plugin.json` (core) and
 * `.claude-plugin/plugin.json` (Claude Code) with the same values in the fields both define, chosen
 * so that they load in both. The check reports, per file, what loads in one form only: a core
 * manifest that fails core §5; a field whose two values differ; a name that is not kebab-case (a `.`
 * draws Claude Code's warning, J18.19); an `author` without `name` and a `homepage` that is no
 * absolute URL, which Claude Code refuses; a missing `description`, which its validator warns
 * about; a `version` that is not SemVer; `$schema` or `extensions` in the Claude Code manifest.
 * Claude Code's own rules on its manifest stay `validatePlugin`'s.
 */

export const CORE_MANIFEST_FILE = 'plugin.json';
export const CLAUDE_MANIFEST_FILE = '.claude-plugin/plugin.json';

/** The fields both forms define (§18.3.1). */
const SHARED = [
  'name',
  'displayName',
  'version',
  'description',
  'author',
  'homepage',
  'repository',
  'license',
  'keywords',
] as const;

const KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

function isAbsoluteUrl(text: string): boolean {
  try {
    return new URL(text).protocol.length > 0;
  } catch {
    return false;
  }
}

function differences(
  core: Readonly<Record<string, unknown>>,
  claude: Readonly<Record<string, unknown>>,
): ValidationIssue[] {
  return SHARED.filter(
    (field) =>
      (Object.hasOwn(core, field) || Object.hasOwn(claude, field)) &&
      canonicalJson(core[field]) !== canonicalJson(claude[field]),
  ).map((field) =>
    errorIn(
      CORE_MANIFEST_FILE,
      field,
      `"${field}" differs from ${CLAUDE_MANIFEST_FILE}: a dual package gives both the same value`,
    ),
  );
}

/** What of the shared values loads in one form only, judged on the core manifest's values. */
function valueIssues(core: Readonly<Record<string, unknown>>): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  const { name, author, homepage, version } = core;
  if (typeof name === 'string' && !KEBAB.test(name)) {
    issues.push(
      warningIn(
        CORE_MANIFEST_FILE,
        'name',
        `"${name}" is not kebab-case: Claude Code accepts it with a warning`,
      ),
    );
  }
  if (isRecord(author) && typeof author.name !== 'string') {
    issues.push(errorIn(CORE_MANIFEST_FILE, 'author', 'Claude Code requires "author.name"'));
  }
  if (typeof homepage === 'string' && !isAbsoluteUrl(homepage)) {
    issues.push(
      errorIn(
        CORE_MANIFEST_FILE,
        'homepage',
        'Claude Code requires "homepage" to be an absolute URL',
      ),
    );
  }
  if (core.description === undefined) {
    issues.push(
      warningIn(CORE_MANIFEST_FILE, 'description', 'Both forms should carry a "description"'),
    );
  }
  if (typeof version === 'string' && !SEMVER.test(version)) {
    issues.push(
      warningIn(CORE_MANIFEST_FILE, 'version', `"${version}" is not Semantic Versioning`),
    );
  }
  return issues;
}

function claudeOnlyIssues(claude: Readonly<Record<string, unknown>>): ValidationIssue[] {
  return ['$schema', 'extensions']
    .filter((field) => Object.hasOwn(claude, field))
    .map((field) =>
      warningIn(CLAUDE_MANIFEST_FILE, field, `"${field}" belongs in ${CORE_MANIFEST_FILE} only`),
    );
}

/** Checks the two parsed manifests of a dual package (§18.3.1). */
export function checkDualManifests(core: unknown, claude: unknown): readonly ValidationIssue[] {
  const read = readCoreManifest(core);
  const coreIssues = read.issues.map((found) => ({ ...found, file: CORE_MANIFEST_FILE }));
  if (!isRecord(core) || !isRecord(claude)) {
    const which = isRecord(claude) ? CORE_MANIFEST_FILE : CLAUDE_MANIFEST_FILE;
    return [...coreIssues, errorIn(which, '', `${which} is not a JSON object`)];
  }
  return [
    ...coreIssues,
    ...valueIssues(core),
    ...differences(core, claude),
    ...claudeOnlyIssues(claude),
  ];
}
