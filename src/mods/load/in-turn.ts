/**
 * Every change to the set runs after the one before it ends (judgment): a stop of the worker that
 * comes in the middle of `reload`, or a second stop in a row, would otherwise both rewrite `loaded`.
 */
export const inTurn = (): (<T>(run: () => Promise<T>) => Promise<T>) => {
  let tail: Promise<unknown> = Promise.resolve();
  return (run) => {
    const next = tail.then(run);
    tail = next.catch(() => undefined);
    return next;
  };
};
