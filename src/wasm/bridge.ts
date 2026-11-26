/**
 * The one door into the Rust core (docs/RUST.md): `coreModule('<module>')` gives a wrapper the
 * caller of its module's exports in the WebAssembly module `crates/agent-plugins-wasm` builds,
 * which answers what the function returns or throws what it throws. A wrapper module of `src/`
 * keeps its TypeScript signature and calls through here; nothing else of the package knows the
 * module is Rust.
 *
 * The module's bytes are `src/wasm/module.ts`, written by `bun run build:wasm` (not committed):
 * pure data, so the package still reads nothing of the machine it runs on (L3). It is compiled
 * once, when the package loads, and keeps no state between calls. A trap (the core panicked: a
 * bug, RS4) leaves its memory in no known state, so that instance is dropped and the next call
 * gets a fresh one.
 */
import { MODULE } from './module.ts';
import { decodeAnswer, encodeArgs } from './wire.ts';

interface Exports {
  readonly memory: WebAssembly.Memory;
  agent_plugins_alloc(len: number): number;
  agent_plugins_free(ptr: number, len: number): void;
  agent_plugins_call(namePtr: number, nameLen: number, argsPtr: number, argsLen: number): number;
}

const decodeBase64 = (text: string): Uint8Array<ArrayBuffer> => {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
};

const compiled = await WebAssembly.compile(decodeBase64(MODULE));
// The module's exports are the three functions and the memory `Exports` names (its ABI,
// crates/agent-plugins-wasm/src/abi.rs); WebAssembly types them only as `ExportValue`.
const instantiate = (): Exports =>
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the ABI above: abi.rs exports exactly these.
  new WebAssembly.Instance(compiled, {}).exports as unknown as Exports;
let core = instantiate();
const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

/** Copies `bytes` into a buffer of the module's, which the call takes back. */
function put(bytes: Uint8Array): number {
  const at = core.agent_plugins_alloc(bytes.length);
  new Uint8Array(core.memory.buffer, at, bytes.length).set(bytes);
  return at;
}

function answerOf(name: string, args: string): string {
  const nameBytes = encoder.encode(name);
  const argsBytes = encoder.encode(args);
  const nameAt = put(nameBytes);
  const argsAt = put(argsBytes);
  const answer = core.agent_plugins_call(nameAt, nameBytes.length, argsAt, argsBytes.length);
  const length = new DataView(core.memory.buffer).getUint32(answer, true);
  const text = decoder.decode(new Uint8Array(core.memory.buffer, answer + 4, length));
  core.agent_plugins_free(answer, length + 4);
  return text;
}

/** The answer's JSON text, or a fault when the core trapped (its instance is then replaced). */
function answerText(name: string, args: string): string {
  try {
    return answerOf(name, args);
  } catch (error) {
    if (!(error instanceof WebAssembly.RuntimeError)) throw error;
    core = instantiate();
    return JSON.stringify({ fault: `the core trapped: ${error.message}` });
  }
}

/**
 * Calls the core's export `name` (`<module>#<export>`, the module's path under `src/` without
 * `.ts`) with `args`; returns its result or throws an `Error` with the message the TypeScript
 * threw. A fault (an unknown name, arguments that do not decode) is a bug of the package.
 *
 * `T` is the wrapper's declared return type: the core answers the export's TypeScript type,
 * which the package's tests hold every wrapper to, so this is the one place the type is taken
 * on trust.
 */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- T is the wrapper's declared return type (above).
function callCore<T>(name: string, args: readonly unknown[]): T {
  const answer = decodeAnswer(answerText(name, encodeArgs(args)));
  // The core's result is the export's declared type (above); JSON gives it as `unknown`.
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the one place the type is taken on trust (above).
  if ('result' in answer) return answer.result as T;
  if ('throws' in answer) throw new Error(answer.throws);
  throw new Error(
    `agent-plugins: ${name}: ${answer.fault} (a bug in agent-plugins; please report it)`,
  );
}

/**
 * The caller of one module's exports, for its wrapper: `const call = coreModule('format/x')`, then
 * `call('f', a, b)`. `T` is the wrapper's declared return type, taken on trust as `callCore` says.
 */
export const coreModule =
  (module: string) =>
  // oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- as callCore's T: one place, not one per wrapper.
  <T>(name: string, ...args: readonly unknown[]): T =>
    callCore<T>(`${module}#${name}`, args);
