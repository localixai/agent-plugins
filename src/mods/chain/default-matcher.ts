import type { ModMatcher } from '../../protocol/mods-matcher.ts';
import { isRecord } from '../../protocol/is-record.ts';
import type { ChainEntry } from './registry.ts';
import { isUnfiltered } from './match-local.ts';
import { isWildcard } from './wildcard.ts';

/**
 * What a matcher means beyond its own fields (spec §10.13.3, "Other mods' drawings and controls";
 * `Matcher`, claude-code/index.d.ts:7505 of 2.1.330), and the `tool.check` events only a hook naming
 * `op` hears (§10.16.3 item 1).
 */

/** The events whose `plugin` is a default matcher key: left out, it names the hook's own plugin. */
const PLUGIN_KEYED: ReadonlySet<string> = new Set([
  'ui.press',
  'ui.input',
  'ui.select',
  'ui.focus',
  'ui.render',
]);

/** The events raised for a mod's own `$` call with `op` set, heard only by a matcher that names `op`. */
const OP_KEYED: ReadonlySet<string> = new Set(['tool.check', 'tool.judge']);

/** The band's stack: a `component` matcher keeps a hook off it unless it names the list (§10.21.1 item 4). */
const LIST = 'AbovePromptList';

/** `{ plugin: '*' }`: every mod, and an event that carries no `plugin` (J10.12). */
const ANY_PLUGIN = '*';

const names = (matcher: ModMatcher | undefined, field: string): boolean =>
  matcher?.[field] !== undefined;

/** The matcher as its own fields are tested: `plugin: '*'` is no filter of a value. */
export function fieldsTested(matcher: ModMatcher | undefined): ModMatcher | undefined {
  if (matcher?.plugin !== ANY_PLUGIN) return matcher;
  const { plugin: _any, ...rest } = matcher;
  return rest;
}

/** Whether a hook without `plugin` hears the event: a bare wildcard hears every mod, any other its own. */
function ownOrBare(entry: ChainEntry, plugin: unknown): boolean {
  const { event, matcher } = entry.hook;
  if (isWildcard(event) && isUnfiltered(matcher)) return true;
  return plugin === entry.plugin.name;
}

/** Whether a matcher naming another `component` keeps a `ui.render` hook off the band's stack. */
const keptOffList = (matcher: ModMatcher | undefined, component: unknown): boolean =>
  component === LIST && names(matcher, 'component') && matcher?.component !== LIST;

/** Whether an event raised for a mod's own call (`op`) passes a hook whose matcher does not name `op`. */
const opUnnamed = (event: string, matcher: ModMatcher | undefined, op: unknown): boolean =>
  OP_KEYED.has(event) && op !== undefined && !names(matcher, 'op');

/**
 * Whether the hook of `entry` may hear `e` before its matcher's fields are tested: a hook on another
 * mod's control or drawing needs `plugin`; a matcher naming another `component` keeps a `ui.render`
 * hook off `AbovePromptList`; an event with `op` reaches only a matcher that names `op`.
 */
export function admitsByDefault(entry: ChainEntry, event: string, e: unknown): boolean {
  if (!isRecord(e)) return true;
  const { matcher } = entry.hook;
  if (opUnnamed(event, matcher, e.op)) return false;
  if (!PLUGIN_KEYED.has(event)) return true;
  if (keptOffList(matcher, e.component)) return false;
  if (names(matcher, 'plugin') || typeof e.plugin !== 'string') return true;
  return ownOrBare(entry, e.plugin);
}
