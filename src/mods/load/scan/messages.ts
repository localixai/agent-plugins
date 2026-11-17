/**
 * The words of the scan's refusals (plugins/mods/create.md, "Check what Claude Code reads from your
 * mod"), Claude Code 2.1.325's where the specification quotes them (Agent Plugins Extended §10.4.2,
 * Judgment J10.2).
 */

export const NOT_LITERAL = 'the event name passed to on() is not a string literal';
export const SHADOWED = '"on" is declared again (shadowed)';
export const COMPUTED = '(computed)';
export const AS_VALUE = 'is used as a value (a noun of $ bound, passed or read)';
/** oracle 2.1.325: `$` itself bound, destructured or read as a value. */
export const DOLLAR_BOUND = '$ itself is bound to a name (bound, passed, spread, returned or read)';
/** oracle 2.1.325: `$[name]` or `$?.name`. */
export const DOLLAR_COMPUTED = 'a computed or optional member access on $';
export const TELEMETRY_STREAM =
  'its hooks stand on the telemetry stream "anthropic" (a matcher names it, or a telemetry hook names no "to" and so stands on every stream), which is for the plugins built into the CLI; name the collector on each telemetry hook: on("telemetry.log", { to: "collector" }, hook)';
export const unreadableHook = (event: string): string =>
  `the hook passed to on("${event}") is not a function the scan can read`;
