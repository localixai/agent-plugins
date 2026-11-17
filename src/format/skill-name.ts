import { error, warning, type PluginIssue } from './issues.ts';
import type { SkillLayout, SkillOrigin, SkillOriginKind } from './skill-types.ts';
import { codePointLength } from './code-points.ts';

/**
 * How a skill gets its command name (skills.md, "How a skill gets its command name" and
 * "Characters a skill name can't hold"; plugins/components.md, "Skills").
 */

export interface SkillName {
  /** The name the skill goes by: the frontmatter `name`, else the folder's name. */
  readonly name: string;
  /** What the user types after `/`. */
  readonly command: string;
  /**
   * `path` is the field the issue is about: `name` (the frontmatter) or `folder`. A refused name's
   * message is its debug-log line, `[names] <name> is not offered` (the skill does not load) or
   * `[names] <value> is not shown for <skill>` (it loads without that `name`), hidden characters
   * spelled `\u{200B}`; a host writes it to the debug log and counts the lines for the startup
   * notice (`skillNamesRefused`). Any other message is a whole sentence that starts with the field
   * (`folder name uses ...`). An error means the skill does not load; a warning means it loads.
   */
  readonly issues: readonly PluginIssue[];
}

/**
 * Characters "such as" those the docs list: a zero-width space or a control character (Cc, and
 * the zero-width and bidi controls, which are all format characters, Cf), a private-use or
 * unassigned character (Co, Cn, and lone surrogates, Cs), a blank (the Z categories) and `<`/`>`.
 * A plain space is judged by its position, not here.
 */
const REFUSED = /[\p{Cc}\p{Cf}\p{Co}\p{Cn}\p{Cs}\p{Z}<>]/u;

/** Which code points of `text` Claude Code refuses. A space is fine only alone between words. */
function refusedFlags(text: string): { readonly points: string[]; readonly flags: boolean[] } {
  const points = Array.from(text);
  const lastIndex = points.length - 1;
  const flags = points.map((point, index) =>
    point === ' '
      ? index === 0 || index === lastIndex || points[index - 1] === ' ' || points[index + 1] === ' '
      : REFUSED.test(point),
  );
  return { points, flags };
}

const hex = (point: string): string =>
  `U+${(point.codePointAt(0) ?? 0).toString(16).toUpperCase().padStart(4, '0')}`;

/** The refused characters of `text` as code points (never the raw text: it may be invisible). */
export function refusedCharacters(text: string): string[] {
  const { points, flags } = refusedFlags(text);
  return points.filter((_point, index) => flags[index]).map(hex);
}

/** Each refused character becomes one hyphen. */
function hyphenate(text: string): string {
  const { points, flags } = refusedFlags(text);
  return points.map((point, index) => (flags[index] ? '-' : point)).join('');
}

const cannotHold = (field: string, found: readonly string[]): string =>
  `${field} has a character a command name can't hold (${found.join(', ')})`;

/** skills.md, "Characters a skill name can't hold"; errors.md: "or be over 256 characters". */
const MAX_NAME = 256;

/** Whether Claude Code refuses `text` as a name: a character it can't hold, or over 256 code points. */
const isRefused = (text: string): boolean =>
  codePointLength(text) > MAX_NAME || refusedFlags(text).flags.includes(true);

/**
 * `text` as the debug log spells it (errors.md, "Skill or command name refused": "spells out some
 * hidden characters as codes, such as `\u{200B}`"): every refused character but the visible `<`
 * and `>` as its code point, so a blank at either end or a doubled space shows too.
 */
function spelled(text: string): string {
  const { points, flags } = refusedFlags(text);
  return points
    .map((point, index) =>
      flags[index] === true && point !== '<' && point !== '>'
        ? `\\u{${(point.codePointAt(0) ?? 0).toString(16).toUpperCase()}}`
        : point,
    )
    .join('');
}

/** errors.md, "Skill or command name refused": the `[names]` line of a name a skill goes by. */
const notOffered = (name: string): string => `[names] ${spelled(name)} is not offered`;

/** The same, for a frontmatter `name` the skill loads without. */
const notShownFor = (value: string, skill: string): string =>
  `[names] ${spelled(value)} is not shown for ${skill}`;

interface Chosen {
  readonly name: string;
  readonly issues: readonly PluginIssue[];
}

const NAMESPACE = 'anthropic-skills';

/** skills.md, "Names reserved for synced skills": `anthropic-skills` and `anthropic-skills:<name>`. */
export const reservedNamespace = (text: string): string | undefined =>
  text === NAMESPACE || text.startsWith(`${NAMESPACE}:`) ? NAMESPACE : undefined;

/** skills.md, "Choose where skills load": a folder named `synced`, "in any capitalization". */
const reservedFolder = (text: string): string | undefined =>
  text.toLowerCase() === 'synced' ? 'synced' : undefined;

interface Reserved {
  /** The issue's `path`, and the first word of its message. */
  readonly path: 'folder' | 'name';
  readonly label: string;
  readonly text: string | undefined;
  readonly fix: string;
  /** The reserved word of `text`, when it holds one. */
  readonly reserved: (text: string) => string | undefined;
}

/** The error of a reserved name: it refuses the skill (skills.md, "Names reserved for synced skills"). */
function reservedErrors(field: Reserved): PluginIssue[] {
  const { path, label, text, fix } = field;
  const word = text === undefined ? undefined : field.reserved(text);
  if (word === undefined) return [];
  return [
    error(
      path,
      `${label} uses "${word}", a name reserved for skills synced from claude.ai; the skill does not load, ${fix}`,
    ),
  ];
}

/**
 * Outside a plugin (skills.md, "Characters a skill name can't hold"; errors.md, "Skill or command
 * name refused"): a folder name Claude Code refuses keeps the skill from loading, an error; a
 * refused frontmatter `name` is ignored, a warning, and the skill loads under its folder's name. A
 * reserved folder or `name` refuses the skill. The folder `synced` is reserved only where Claude
 * Code authors no `synced` folder of its own: the user and project locations, not a synced skill.
 */
const strict =
  (reservesSyncedFolder: boolean) =>
  (dirName: string, declared: string | undefined): Chosen => {
    const ignored = declared !== undefined && isRefused(declared);
    const issues = [
      ...(isRefused(dirName) ? [error('folder', notOffered(dirName))] : []),
      ...reservedErrors({
        path: 'folder',
        label: 'folder name',
        text: dirName,
        fix: 'rename the folder',
        reserved: (text) =>
          reservedNamespace(text) ?? (reservesSyncedFolder ? reservedFolder(text) : undefined),
      }),
      ...(ignored ? [warning('name', notShownFor(declared, dirName))] : []),
      ...reservedErrors({
        path: 'name',
        label: 'name',
        text: declared,
        fix: 'edit the name: line',
        reserved: reservedNamespace,
      }),
    ];
    return { name: ignored ? dirName : (declared ?? dirName), issues };
  };

/**
 * In a plugin the skill always loads. A folder's refused characters become hyphens in the command
 * name; a refused frontmatter `name` is not used, with its `[names]` line (spec/judgments.md: the
 * folder names the skill). A long folder name is kept: the docs give a plugin folder only the
 * hyphen rule.
 */
function lenient(dirName: string, declared: string | undefined): Chosen {
  if (declared !== undefined && !isRefused(declared)) return { name: declared, issues: [] };
  const issues: PluginIssue[] = [];
  const name = hyphenate(dirName);
  if (declared !== undefined) issues.push(warning('name', notShownFor(declared, name)));
  if (name !== dirName) {
    issues.push(
      warning(
        'folder',
        `${cannotHold('folder name', refusedCharacters(dirName))}; each becomes a hyphen in the command name (${name})`,
      ),
    );
  }
  return { name, issues };
}

/**
 * The command name of a skill named by itself (the plugin root, or a manifest path to its folder),
 * oracle 2.1.319 (spec/judgments.md): the plugin's own `<plugin>:` prefix is dropped, then each
 * UTF-16 unit outside `[A-Za-z0-9_-]` is a hyphen, so `Plan Review` is `Plan-Review`, `other:x` is
 * `other-x` and an emoji is two. `claude plugin details` prints the name as written.
 */
export function selfNamedCommand(name: string, plugin: string): string {
  const own = name.startsWith(`${plugin}:`) ? name.slice(plugin.length + 1) : name;
  // No `u` flag on purpose: Claude Code replaces UTF-16 units, not code points.
  return own.replaceAll(/[^\w-]/g, '-');
}

interface NameRule {
  /** What the command starts with. */
  readonly prefix: (origin: SkillOrigin) => string;
  readonly choose: (dirName: string, declared: string | undefined) => Chosen;
}

export const pluginPrefix = (origin: SkillOrigin): string =>
  'plugin' in origin ? `${origin.plugin}:` : '';

const RULES: { readonly [K in SkillOriginKind]: NameRule } = {
  plugin: { prefix: pluginPrefix, choose: lenient },
  managed: { prefix: () => '', choose: strict(true) },
  user: { prefix: () => '', choose: strict(true) },
  project: { prefix: () => '', choose: strict(true) },
  synced: { prefix: () => `${NAMESPACE}:`, choose: strict(false) },
  // Judgment (spec/judgments.md): a host's own skill takes the rules of a skill outside a plugin,
  // `anthropic-skills` included ("reserved in every session"); no folder of its is scanned.
  bundled: { prefix: () => '', choose: strict(false) },
};

/**
 * Resolve the name and the command of a skill from where it sits and its frontmatter `name`
 * (`declared`, as read; undefined when absent). A `name` that already starts with the prefix is
 * not prefixed again (v2.1.246 and later). Never throws: a refusal is an error issue, and `name`
 * and `command` are then the text as written, for the host to report.
 */
export function resolveSkillName(layout: SkillLayout, declared: string | undefined): SkillName {
  const rule = RULES[layout.origin.kind];
  const { name, issues } = rule.choose(layout.dirName, layout.fixedName ? undefined : declared);
  const prefix = rule.prefix(layout.origin);
  return { name, command: name.startsWith(prefix) ? name : `${prefix}${name}`, issues };
}
