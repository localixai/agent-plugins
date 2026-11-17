import { error, type PluginIssue } from '../../format/issues.ts';
import type { MarketplaceEntry } from '../../format/marketplace/types.ts';

/**
 * What a marketplace entry says that a core package does not read (Agent Plugins Extended §18.1.5
 * item 2, J18.15): its component fields and `strict: false` are Claude Code configuration, and a
 * core package has fixed locations and nothing configured beside them (core §5.1, §6.1). Each is
 * reported in the package's errors and the package loads as the core says; the entry's metadata
 * applies as for any entry. Judgment: every component key the entry reader keeps is reported, the
 * chapter's eight and `userConfig`, `channels`, `settings` and `workflows`, none of which a core
 * package reads either.
 */
export function coreEntryIssues(entry: MarketplaceEntry | undefined): readonly PluginIssue[] {
  if (entry === undefined) return [];
  const fields = Object.keys(entry.components).map((field) =>
    error(
      field,
      `marketplace entry "${entry.name}": ${field} does not apply to a core package, which loads only its own fixed locations`,
    ),
  );
  const strict = entry.strict
    ? []
    : [
        error(
          'strict',
          `marketplace entry "${entry.name}": strict: false does not apply to a core package, whose plugin.json is always its manifest`,
        ),
      ];
  return [...fields, ...strict];
}
