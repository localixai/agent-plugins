/**
 * Reading a validated `hookSpecificOutput` without a cast: the schema already checked every field,
 * so these only look a field up and narrow it.
 */

const valueIn = (source: object | undefined, key: string): unknown =>
  source === undefined ? undefined : Object.entries(source).find(([name]) => name === key)?.[1];

export const stringIn = (source: object | undefined, key: string): string | undefined => {
  const value = valueIn(source, key);
  return typeof value === 'string' ? value : undefined;
};
