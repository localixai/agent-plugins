/**
 * Reads one field of an event's facts, for the rules that depend on one (`source`, `tool_name`,
 * `prompt`, `agent_id`). Only the facts' own fields count, never one inherited through the
 * prototype.
 */
export const factOf = (facts: object, key: string): unknown =>
  Object.entries(facts).find(([name]) => name === key)?.[1];
