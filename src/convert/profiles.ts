/**
 * The tool matchers that select exactly one core tool profile (Agent Plugins Extended §18.3.4,
 * §18.4.3 item 4, the table of §18.2.7): a Claude Code matcher converts when it names one profile's
 * tools, and a core handler that a converted adapter limits to one profile converts back to that
 * matcher.
 */

export const PROFILE_NAMES = [
  'shell',
  'file.read',
  'file.modify',
  'web.fetch',
  'web.search',
  'mcp',
] as const;
export type ProfileName = (typeof PROFILE_NAMES)[number];

/** Each profile's Claude Code tools, as the matcher of §18.3.4 lists them. */
const TOOLS: { readonly [P in ProfileName]: readonly string[] } = {
  shell: ['Bash', 'PowerShell'],
  'file.read': ['Read'],
  'file.modify': ['Edit', 'Write', 'NotebookEdit'],
  'web.fetch': ['WebFetch'],
  'web.search': ['WebSearch'],
  mcp: ['mcp__.*'],
};

export const isProfileName = (value: string | undefined): value is ProfileName =>
  PROFILE_NAMES.some((name) => name === value);

/** The matcher that selects a profile's tools. */
export const matcherOf = (profile: ProfileName): string => TOOLS[profile].join('|');

/** A matcher that matches every tool (hooks.md, "Matcher patterns": omitted, `""` or `*`). */
export const matchesEverything = (matcher: string | undefined): boolean =>
  matcher === undefined || matcher === '' || matcher === '*';

const sameSet = (a: readonly string[], b: readonly string[]): boolean =>
  a.length === b.length && a.every((name) => b.includes(name));

/** The one profile a matcher selects, in any order of its names; `undefined` for any other matcher. */
export function profileOfMatcher(matcher: string): ProfileName | undefined {
  const names = matcher.split('|');
  return PROFILE_NAMES.find((profile) => sameSet(TOOLS[profile], names));
}
