/** The matcher of a hook (`on(event, hook, matcher)`): a leaf, so a runtime in an isolate can name it. */

/**
 * A value a matcher field can hold: a literal, a regular expression (serialisable as
 * `{ $regexp, flags }`), or an object matched as a partial of an object field (§10.13.3).
 */
export type ModMatcherValue =
  | string
  | number
  | boolean
  | null
  | RegExp
  | { $regexp: string; flags?: string }
  | ModMatcher;
/** A filter on an event's fields (events.md, "Filter which events a hook handles"). */
export interface ModMatcher {
  readonly [field: string]: ModMatcherValue | readonly ModMatcherValue[] | undefined;
}
