/**
 * The values of one call the host keeps by identity (docs/RUST.md, "The boundary"): its ports,
 * callbacks, signals and promises, each crossing as a number. Numbers are never reused, so a handle's call
 * reads the table of the call that made the handle as its parent, and a core promise's loop the
 * table of the call that answered it: a port the core keeps in a handle or a promise is still found. The tables are dropped with the calls and handles that hold them.
 */
import type { HostValues } from './wire.ts';

/** A call's host values, and the signals among them that aborted since it last asked. */
export interface HostTable extends HostValues {
  /** Whether nothing was kept: the call can take the plain path, with no loop. */
  readonly empty: () => boolean;
  /** The numbers of the signals that aborted and were not reported yet, its parent's too. */
  readonly aborted: () => readonly number[];
}

/**
 * What a table makes of the core's values: a handle, the object whose methods call the core; a
 * core promise, the promise that drives its task (`drive.ts`).
 */
export interface Makers {
  readonly handle: (
    handle: number,
    methods: { readonly sync: readonly string[]; readonly async: readonly string[] },
    hosts: HostTable,
  ) => object;
  readonly promise: (task: number, hosts: HostTable) => Promise<unknown>;
}

let lastNumber = 0;

/** A new table for one call; `parent` is the table of the call that made the handle it calls. */
export function hostTable(makers: Makers, parent?: HostTable): HostTable {
  const values = new Map<number, unknown>();
  const numbers = new Map<object, number>();
  const signals = new Map<number, AbortSignal>();
  const adopted = new Set<HostValues>();
  const fromAdopted = (number: number): unknown => {
    for (const other of adopted) {
      const found = other.find(number);
      if (found !== undefined) return found;
    }
    return undefined;
  };
  const table: HostTable = {
    keep(value) {
      const known = numbers.get(value);
      if (known !== undefined) return known;
      lastNumber += 1;
      values.set(lastNumber, value);
      numbers.set(value, lastNumber);
      if (value instanceof AbortSignal) signals.set(lastNumber, value);
      return lastNumber;
    },
    find: (number) =>
      values.has(number) ? values.get(number) : (fromAdopted(number) ?? parent?.find(number)),
    adopt(other) {
      if (other !== table) adopted.add(other);
    },
    handle: (handle, sync, async) => makers.handle(handle, { sync, async }, table),
    promise: (task) => makers.promise(task, table),
    empty: () => values.size === 0 && adopted.size === 0,
    aborted() {
      const now = [...signals].filter(([, signal]) => signal.aborted).map(([number]) => number);
      for (const number of now) signals.delete(number);
      return [...now, ...(parent?.aborted() ?? [])];
    },
  };
  return table;
}
