import { selects } from '../../chain/select.ts';

/**
 * The hooks that can refuse an action, which `claude plugin validate` reports with or without a
 * `.catch` handler (create.md: "one line for each hook that can refuse an action, such as
 * `gating hook without .catch: tool.call`"). The docs name five where they describe the handler
 * (events.md, "Handle a hook that fails"); Agent Plugins Extended §10.4.3 adds the telemetry events
 * and a glob or negation that selects one of them.
 */
const GATING_EVENTS: readonly string[] = [
  'tool.call',
  'tool.check',
  'prompt.submit',
  'config.set',
  'plugin.register',
  'telemetry.log',
  'telemetry.mark',
];

/** Whether a hook on `pattern` (a name, a glob or a negation) can refuse an action. */
export const isGatingPattern = (pattern: string): boolean =>
  GATING_EVENTS.some((event) => selects(pattern, event));
