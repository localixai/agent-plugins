/**
 * Promises as values across the Rust core (docs/RUST.md, "Effects", item 7): the host's promise
 * crosses as its number and the core awaits it, a rejection caught there; a promise of the core's
 * in an answer is a promise here that drives the core's task, its first need started before the
 * answer is returned, and two of them settle apart, each when its own host work does.
 */
import { describe, expect, test } from 'bun:test';
import { coreModule } from '../../src/wasm/bridge.ts';

const probe = coreModule('registry/probe');

interface Later {
  readonly done: Promise<unknown>;
}

describe("the host's promise", () => {
  test('is awaited by the core, in the order given, whichever settles first', async () => {
    const { promise: slow, resolve } = Promise.withResolvers<string>();
    const settling = probe.async<unknown[]>('settled', [slow, Promise.resolve({ a: 1 })]);
    resolve('first');
    expect(await settling).toEqual(['first', { a: 1 }]);
  });

  test('that rejects is caught by the core (catch_host), and the others still count', async () => {
    const settled = probe.async<unknown[]>('settled', [
      Promise.reject(new Error('process vanished')),
      Promise.resolve('kept'),
    ]);
    expect(await settled).toEqual(['rejected: process vanished', 'kept']);
  });
});

describe("the core's promise", () => {
  test('starts its host work before the answer that holds it is returned', async () => {
    const { promise, resolve } = Promise.withResolvers<string>();
    let calls = 0;
    const later = probe<Later>('later', () => {
      calls += 1;
      return promise;
    });
    expect(calls).toBe(1);
    expect(later.done).toBeInstanceOf(Promise);
    resolve('done');
    expect(await later.done).toBe('done');
  });

  test("rejects with the very value the host's work threw", async () => {
    const failure = new RangeError('gone');
    const later = probe<Later>('later', () => Promise.reject(failure));
    expect(await later.done.catch((error: unknown) => error)).toBe(failure);
  });

  test('two settle apart, each when its own host work does', async () => {
    const first = Promise.withResolvers<string>();
    const second = Promise.withResolvers<string>();
    const order: string[] = [];
    const one = probe<Later>('later', () => first.promise);
    const two = probe<Later>('later', () => second.promise);
    const watched = [
      one.done.then((value) => order.push(String(value))),
      two.done.then((value) => order.push(String(value))),
    ];
    second.resolve('second');
    await two.done;
    expect(order).toEqual(['second']);
    first.resolve('first');
    await Promise.all(watched);
    expect(order).toEqual(['second', 'first']);
  });

  test('given back to the core is a promise of the host, awaited there', async () => {
    const later = probe<Later>('later', () => Promise.resolve('through'));
    expect(await probe.async<unknown[]>('settled', [later.done])).toEqual(['through']);
  });
});
