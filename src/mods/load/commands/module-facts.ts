import { registerUses, type RegisterUses } from '../register-event.ts';
import { combineScans, type ModScan } from '../scan/index.ts';
import { hookLabel } from '../scan/uses.ts';

/**
 * One item of a hooks module's `modules` in the validator's `--json` result (Agent Plugins Extended
 * §17.13.4: "one item per hooks module with fields such as `hooks` and `uses`"; 2.1.326 on). Judgment
 * (spec/judgments.md): `module` is its path from the plugin root, as `e.provides.modules` names it,
 * `hooks` the `hooks:` line's entries and `uses` what `plugin.register` hands a policy mod as `e.uses`.
 */
export interface ModuleFacts {
  readonly module: string;
  readonly hooks: readonly string[];
  readonly uses: RegisterUses;
}

export const moduleFacts = (entry: string, scan: ModScan): ModuleFacts => ({
  module: entry,
  hooks: scan.hooks.map(hookLabel),
  uses: registerUses(combineScans([scan])),
});

/** The module's hooks that can refuse an action, as each `--json` result lists them (J17.15). */
export const gatingHooksOf = (written: string, scan: ModScan) =>
  scan.hooks
    .filter((hook) => hook.isGating)
    .map((hook) => ({
      module: written,
      pattern: hook.event,
      hook: hookLabel(hook),
      hasCatch: hook.hasCatch,
    }));
