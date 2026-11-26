/**
 * The one door into the Rust core (docs/RUST.md): `coreModule('<module>')` gives a wrapper the
 * caller of its module's exports in the WebAssembly module `crates/agent-plugins-wasm` builds
 * (`instance.ts`), which answers what the function returns or throws what it throws. A wrapper
 * module of `src/` keeps its TypeScript signature and calls through here; nothing else of the
 * package knows the module is Rust.
 *
 * `call('f', …)` answers at once, for a function the TypeScript declares synchronous (a callback
 * among its arguments is called as the core asks); `call.async('f', …)` returns a promise, for
 * one that is `async` or takes a port or an `AbortSignal` (`drive.ts`).
 */
import { callAsync, callSync } from './drive.ts';

/** A module's caller: `call(name, …args)`, and `call.async(name, …args)` for a promise. */
export interface CoreCall {
  // oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- T is the wrapper's declared return type (drive.ts, resultOf).
  <T>(name: string, ...args: readonly unknown[]): T;
  // oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as above.
  readonly async: <T>(name: string, ...args: readonly unknown[]) => Promise<T>;
}

/**
 * The caller of one module's exports, for its wrapper: `const call = coreModule('format/x')`, then
 * `call('f', a, b)` or `call.async('g', files, path)`. `T` is the wrapper's declared return type,
 * taken on trust (`drive.ts`, `resultOf`).
 */
export const coreModule = (module: string): CoreCall =>
  Object.assign(
    // oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as CoreCall's T.
    <T>(name: string, ...args: readonly unknown[]): T => callSync<T>(`${module}#${name}`, args),
    {
      // oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as CoreCall's T.
      async: <T>(name: string, ...args: readonly unknown[]): Promise<T> =>
        callAsync<T>(`${module}#${name}`, args),
    },
  );
