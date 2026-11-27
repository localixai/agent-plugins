/**
 * The environment variables a component's process receives (the "Exported to the process" column).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/exported_environment.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { VariableComponent } from './field-map.ts';
import type { SubstitutionScope } from './scope.ts';
import type { Substituter } from './substituter.ts';

const call = coreModule('loading/resolve/exported-environment');

/**
 * The environment variables a component's process receives (the "Exported to the process" column):
 * the host adds them to the process it starts. Paths are as the host reports them, native: a
 * Windows process reads backslashes, except a Git Bash hook, which `resolveHookCommand` gives the
 * forward-slash form (spec J3.6). The data folder is asked for only by a component that exports it; pass the
 * `substituter` of the component's strings so it is asked once for both.
 */
export const exportedEnvironment = (
  component: VariableComponent,
  scope: SubstitutionScope,
  substitution?: Pick<Substituter, 'variable'>,
): Promise<Record<string, string>> =>
  call.async('exportedEnvironment', component, scope, substitution);
