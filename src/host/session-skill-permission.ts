import type { PermissionBehavior } from '../protocol/official.ts';
import { decide, parseRule, type DecideContext, type Decision } from '../rules/index.ts';
import type { ShellPermission } from '../skills/index.ts';

/** `decide` has a classifier answer the kit cannot give: auto mode treats it as an ask (it defers). */
const BEHAVIOR_OF: { readonly [O in Decision['outcome']]: PermissionBehavior } = {
  allow: 'allow',
  ask: 'ask',
  deny: 'deny',
  classify: 'ask',
};

/**
 * The default answer for a skill's command (skills.md, "Permission checks on injected commands"):
 * `decide` over the session's rules with the skill's `allowed-tools` added as allow rules, so "Deny
 * and ask rules still override `allowed-tools`". A grant that does not parse grants nothing.
 */
export function rulesPermission(context: DecideContext): ShellPermission {
  return (call) => {
    const grants = call.grants.flatMap(
      (text) => parseRule(text, { behavior: 'allow', source: { kind: 'session' } }).rule ?? [],
    );
    const decision = decide(
      { toolName: call.tool, input: { command: call.command } },
      { ...context, rules: [...context.rules, ...grants] },
    );
    return BEHAVIOR_OF[decision.outcome];
  };
}
