import { isRecord } from '../../protocol/is-record.ts';

/**
 * Client extension data (core §8, §8.1): an `extensions` field is an object whose members are
 * extension namespaces and whose values are objects. The kit checks that shape only: a namespace's
 * contents belong to the client that owns it, and the members of a namespace the host does not
 * implement are ignored without validating their values.
 */

/** The data of the namespaces a host implements, as written; namespaces it does not implement are dropped. */
export type CoreExtensions = Readonly<Record<string, Readonly<Record<string, unknown>>>>;

/** Why an `extensions` value breaks core §8.1, or `undefined` when its shape is right. */
export function extensionsProblem(value: unknown): string | undefined {
  if (!isRecord(value)) return 'must be an object keyed by extension namespace';
  const bad = Object.entries(value).find(([, data]) => !isRecord(data));
  return bad === undefined ? undefined : `namespace "${bad[0]}" must hold an object`;
}

/**
 * The members of a valid `extensions` value that the host implements (core §8.1: the others are
 * ignored). `undefined` when nothing implemented is there, so a result carries no empty object.
 */
export function implementedExtensions(
  value: unknown,
  implemented: readonly string[],
): CoreExtensions | undefined {
  if (!isRecord(value)) return undefined;
  const kept = Object.entries(value).filter(
    (entry): entry is [string, Record<string, unknown>] =>
      implemented.includes(entry[0]) && isRecord(entry[1]),
  );
  return kept.length === 0 ? undefined : Object.fromEntries(kept);
}
