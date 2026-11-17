import { parseFrontmatter } from '../format/frontmatter.ts';
import { findInjections } from '../skills/shell-scan.ts';

/**
 * What Claude Code expands in a skill's body and a core client shows as written (Agent Plugins
 * Extended §18.3.2 item 3, §18.4.2 item 2, §18.4.3 item 2): argument placeholders (`$ARGUMENTS`,
 * `$ARGUMENTS[N]`, `$N`, skills.md "Available string substitutions"), the `${CLAUDE_*}` and
 * `${user_config.*}` variables, and injected commands (`` !`cmd` `` and ```` ```! ````, "Inject
 * dynamic context"). A placeholder after a single backslash is escaped and stays text, as
 * `skills/substitute.ts` reads it; `$name` expands only for names a Claude Code `arguments` field
 * declares, which no core skill has.
 */

const ARGUMENT = /\\?\$(?:ARGUMENTS\[\d+\]|ARGUMENTS(?![A-Za-z0-9_])|\d+)/g;
const VARIABLE =
  /\$\{(?:CLAUDE_(?:SKILL_DIR|SESSION_ID|EFFORT|PLUGIN_ROOT|PLUGIN_DATA|PROJECT_DIR)|user_config\.[^}]+)\}/g;

function argumentPlaceholders(body: string): string[] {
  return [...body.matchAll(ARGUMENT)]
    .filter((found) => !found[0].startsWith('\\') || body.charAt(found.index - 1) === '\\')
    .map((found) => found[0].replace(/^\\/, ''));
}

const unique = (values: readonly string[]): string[] => [...new Set(values)];

/** Each kind of expanded text the body holds, as a phrase that names it, in a fixed order. */
export function expandedFeatures(text: string): readonly string[] {
  const { body } = parseFrontmatter(text);
  const placeholders = unique(argumentPlaceholders(body));
  const variables = unique([...body.matchAll(VARIABLE)].map((found) => found[0]));
  const injections = findInjections(body).map((found) => found.command.trim());
  return [
    ...(placeholders.length === 0 ? [] : [`argument placeholders ${placeholders.join(', ')}`]),
    ...(variables.length === 0 ? [] : [`variables ${variables.join(', ')}`]),
    ...(injections.length === 0 ? [] : [`injected commands (${injections.join('; ')})`]),
  ];
}
