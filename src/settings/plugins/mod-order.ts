/**
 * The order the organization's and the user's own mods run in (settings-reference.md,
 * `prependPlugins` and `appendPlugins`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/mod_order.rs`,
 * docs/RUST.md).
 */
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/plugins/mod-order');

/** How the session signs in; the host knows it, the kit never does (settings-reference.md, `prependPlugins`). */
export type PolicyAccount = 'claudeaiTeamEnterprise' | 'claudeaiPersonal' | 'console';

/** The two machine facts the ordering keys depend on. */
export interface ModOrderContext {
  /** Any admin-controlled source of settings is present on the machine. */
  readonly hasManagedSettings: boolean;
  /** `claudeaiTeamEnterprise` only for a Team or Enterprise `/login`, not for a long-lived token. */
  readonly account: PolicyAccount;
}

export interface SkippedPolicyMod {
  readonly id: string;
  readonly reason: string;
}

/** Where the built-in guard sits: at the position a managed list gives it, outermost by default, or not loaded. */
export type GuardPlacement = 'listed' | 'outermost' | 'absent';

export interface ModOrderResult {
  /** Run before every mod a user installs, in this order. The built-in guard is here when listed. */
  readonly prepend: readonly string[];
  /** Run after every mod a user installs, in this order. */
  readonly append: readonly string[];
  /**
   * The organization's mods that no list names: they run right after `prepend` and before every
   * mod a user installs, in the order `orgMods` holds them (plugins/mods/events.md, "The order mods
   * run in").
   */
  readonly unlisted: readonly string[];
  readonly skipped: readonly SkippedPolicyMod[];
  readonly guard: { readonly loads: boolean; readonly placement: GuardPlacement };
}

/**
 * The order the organization's and the user's own mods run in (settings-reference.md,
 * `prependPlugins` and `appendPlugins`; plugins/mods/admin.md, "Install your organization's mods and
 * set the order"). `orgMods` holds the plugins the host has checked to meet the two conditions
 * the settings cannot show: the marketplace is a directory on the machine and lists the plugin by a
 * relative path, so it loads in place. The kit adds the third, a managed `enabledPlugins` `true`.
 *
 * Judgment (spec/judgments.md): the organization's mods that no list names are `unlisted`, in the
 * order of `orgMods`, a repeated id keeps its first place, the two spellings of the guard
 * are one id, text that is no plugin id and a guard in a user list are skipped with a reason, and a
 * managed layer counts as managed settings whatever the host says.
 */
export const modOrder = (
  view: PolicyView,
  context: ModOrderContext,
  orgMods: ReadonlySet<string>,
): ModOrderResult =>
  call(
    'modOrder',
    view,
    { hasManagedSettings: context.hasManagedSettings, account: context.account },
    orgMods,
  );
