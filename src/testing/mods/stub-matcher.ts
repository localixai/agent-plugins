import { matchesEvent } from '../../mods/chain/match-local.ts';
import type { ModMatcher } from '../../protocol/mods-matcher.ts';
import type { RenderSite } from '../../protocol/mods-ui-names.ts';
import { isRecord } from '../../protocol/is-record.ts';

/** `{ plugin: '*' }`: every mod, and an event that carries no `plugin` (§10.13.3 item 2). */
const ANY_PLUGIN = '*';

/** The site where a stub needs `plugin` to answer: one mod's row of the band. */
const BAND_ROW: RenderSite = 'AbovePrompt';

/** Whether `e` is one mod's row of the band, with `e.plugin` naming the mod. */
const isBandRow = (e: unknown): boolean =>
  isRecord(e) && e.component === BAND_ROW && typeof e.plugin === 'string';

/**
 * Whether a stub registered with `matcher` answers `e`: every field the matcher names lets the
 * event's through, `plugin: '*'` every mod; and at the band, from 2.1.323, a stub without `plugin`
 * does not answer a mod's row (Agent Plugins Extended §10.12: "for the band from 2.1.323 the test's
 * hook needs `{ plugin: '<mod>' }` for the stub to answer it").
 */
export function stubAdmits(matcher: ModMatcher | undefined, e: unknown): boolean {
  const plugin = matcher?.plugin;
  if (plugin === undefined && isBandRow(e)) return false;
  if (plugin !== ANY_PLUGIN) return matchesEvent(matcher, e);
  const { plugin: _any, ...rest } = matcher ?? {};
  return matchesEvent(rest, e);
}
