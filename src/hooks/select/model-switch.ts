import { coreModule } from '../../wasm/bridge.ts';
import type { ModelCatalog } from '../contracts.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/model_switch.rs`,
 * docs/RUST.md).
 */
const call = coreModule('hooks/select/model-switch');

/**
 * The canonical name of the model a switch goes to, which `PreModelSwitch` and `PostModelSwitch`
 * matchers compare against: an alias, a dated id and a provider id all resolve to the one name
 * (the catalog's job, the kit holds no model list). `undefined` when no canonical name can be
 * determined, so every hook of the event runs, whatever its matcher.
 */
export const canonicalMatcherModel = (
  toModel: string,
  catalog?: ModelCatalog,
): string | undefined => call('canonicalMatcherModel', toModel, catalog);
