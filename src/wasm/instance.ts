/**
 * The WebAssembly module `crates/agent-plugins-wasm` builds, and its ABI as text in, text out
 * (`crates/agent-plugins-wasm/src/abi.rs`): a call, a task's start, resume and cancel, a handle's
 * release.
 *
 * The module's bytes are `src/wasm/module.ts`, written by `bun run build:wasm` (not committed):
 * pure data, so the package still reads nothing of the machine it runs on (L3). It is compiled
 * once, when the package loads. A trap (the core panicked: a bug, RS4) leaves its memory in no
 * known state, so that instance is dropped and the next call gets a fresh one; the tasks that
 * waited in it end with a fault, and its handles are gone (`generation` tells them apart).
 */
import { MODULE } from './module.ts';

interface Exports {
  readonly memory: WebAssembly.Memory;
  agent_plugins_alloc(len: number): number;
  agent_plugins_free(ptr: number, len: number): void;
  agent_plugins_call(namePtr: number, nameLen: number, argsPtr: number, argsLen: number): number;
  agent_plugins_start(namePtr: number, nameLen: number, argsPtr: number, argsLen: number): number;
  agent_plugins_resume(task: number, answersPtr: number, answersLen: number): number;
  agent_plugins_release(handle: number): void;
  agent_plugins_cancel(task: number): void;
}

const decodeBase64 = (text: string): Uint8Array<ArrayBuffer> => {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
};

const compiled = await WebAssembly.compile(decodeBase64(MODULE));
// The module's exports are the functions and the memory `Exports` names (its ABI,
// crates/agent-plugins-wasm/src/abi.rs); WebAssembly types them only as `ExportValue`.
const instantiate = (): Exports =>
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- the ABI above: abi.rs exports exactly these.
  new WebAssembly.Instance(compiled, {}).exports as unknown as Exports;
let core = instantiate();
let generations = 0;
const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

/** Which instance answers now: a handle made by an earlier one is gone with it. */
export const generation = (): number => generations;

/** Copies `text` into a buffer of the module's, which the call takes back: its address and length. */
function put(text: string): readonly [number, number] {
  const bytes = encoder.encode(text);
  const at = core.agent_plugins_alloc(bytes.length);
  new Uint8Array(core.memory.buffer, at, bytes.length).set(bytes);
  return [at, bytes.length];
}

/** The text of an answer's buffer, which is given back. */
function taken(answer: number): string {
  const length = new DataView(core.memory.buffer).getUint32(answer, true);
  const text = decoder.decode(new Uint8Array(core.memory.buffer, answer + 4, length));
  core.agent_plugins_free(answer, length + 4);
  return text;
}

/** `run`'s text, or `trapped`'s when the core trapped (its instance is then replaced). */
function guarded(run: () => string, trapped: (why: string) => string): string {
  try {
    return run();
  } catch (error) {
    if (!(error instanceof WebAssembly.RuntimeError)) throw error;
    core = instantiate();
    generations += 1;
    return trapped(`the core trapped: ${error.message}`);
  }
}

const fault = (why: string): string => JSON.stringify({ fault: why });
const faultStep = (why: string): string => JSON.stringify({ done: { fault: why } });

/** The answer's JSON text of a call (`registry::call`). */
export const callText = (name: string, args: string): string =>
  guarded(() => {
    const [nameAt, nameLength] = put(name);
    const [argsAt, argsLength] = put(args);
    return taken(core.agent_plugins_call(nameAt, nameLength, argsAt, argsLength));
  }, fault);

/** The first step's JSON text of a task (`registry::start_json`). */
export const startText = (name: string, args: string): string =>
  guarded(() => {
    const [nameAt, nameLength] = put(name);
    const [argsAt, argsLength] = put(args);
    return taken(core.agent_plugins_start(nameAt, nameLength, argsAt, argsLength));
  }, faultStep);

/** The next step's JSON text of task `task`, given the host's answers (`registry::resume_json`). */
export const resumeText = (task: number, answers: string): string =>
  guarded(() => {
    const [at, length] = put(answers);
    return taken(core.agent_plugins_resume(task, at, length));
  }, faultStep);

/** Drops the core's task `task`, which no loop will resume, unless its instance is gone. */
export function cancel(task: number, madeIn: number): void {
  if (madeIn === generations) core.agent_plugins_cancel(task);
}

/** Forgets the core's handle `handle`, unless the instance that made it is gone. */
export function release(handle: number, madeIn: number): void {
  if (madeIn === generations) core.agent_plugins_release(handle);
}
