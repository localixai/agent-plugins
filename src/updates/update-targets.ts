import { coreModule } from '../wasm/bridge.ts';

/**
 * "Update plugins now" (plugins/install.md): what each way of updating touches. Pure: the host
 * refreshes listings and updates plugins; the kit says which, and which it leaves to the user.
 * "There's no command that updates every plugin at once", so no request here asks for that.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/update_targets.rs`, docs/RUST.md).
 */
const call = coreModule('updates/update-targets');

/** A plugin the user installed, with what decides whether a bulk update may touch it. */
export interface InstalledForUpdate {
  /** `name@marketplace`. */
  readonly id: string;
  readonly marketplace: string;
  /** Its marketplace entry has a `command` source. */
  readonly commandSource?: boolean;
  /** Its marketplace entry sets a `headersHelper` command. */
  readonly headersHelper?: string;
}

export type UpdateRequest =
  /** `Update now` on the Installed tab, or `claude plugin update <plugin>@<marketplace>`. */
  | { readonly kind: 'plugin'; readonly id: string }
  /** `Update marketplace` on the Marketplaces tab. */
  | { readonly kind: 'marketplace-tab'; readonly marketplace: string }
  /** `claude plugin marketplace update [name]`, and the in-session command that opens the tab: the listing only. */
  | { readonly kind: 'marketplace-listing'; readonly marketplace?: string };

export type LeftReason = 'command-source' | 'headers-helper';

export interface LeftToUser {
  readonly id: string;
  readonly reason: LeftReason;
  /** "you update it from its view on the Installed tab or with `claude plugin update <plugin>@<marketplace>`". */
  readonly instruction: string;
}

export interface UpdatePlan {
  /** Marketplaces whose listing is refreshed. */
  readonly refresh: readonly string[];
  /** Plugin ids updated. */
  readonly update: readonly string[];
  /** Plugins a bulk update left for the user to update themselves. */
  readonly leftToUser: readonly LeftToUser[];
}

/**
 * What `request` does. Judgments (spec/judgments.md): a single-plugin update names no marketplace
 * refresh; a plugin that is not installed has nothing to update; a marketplace that is not
 * registered has nothing to refresh; a plugin with both a
 * `headersHelper` and a `command` source is reported as a `headersHelper` one.
 */
export const planUpdate = (
  request: UpdateRequest,
  installed: readonly InstalledForUpdate[],
  registered: readonly string[],
): UpdatePlan => call('planUpdate', request, installed, registered);
