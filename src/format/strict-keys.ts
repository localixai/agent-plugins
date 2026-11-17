import { z } from 'zod';

/**
 * A strict object as Claude Code's schema check treats one (plugins/manifest-reference.md,
 * "Unrecognized fields": "An unknown key inside one is an error"): an unknown key is reported as
 * `Unrecognized key: "<key>"` (`Unrecognized keys: "a", "b"`) at the object, beside the problems of
 * the known fields rather than instead of them, so inside a union the object's own path is kept
 * (oracle 2.1.325: `lspServers.go: Unrecognized key: "foo"`, not a bare `Invalid input` at
 * `lspServers`). Zod's `strictObject` aborts on an unknown key, which a union then hides.
 */
export function closedObject<const Shape extends z.ZodRawShape>(shape: Shape) {
  const known = new Set(Object.keys(shape));
  return z.looseObject(shape).check((context) => {
    const keys = Object.keys(context.value).filter((key) => !known.has(key));
    if (keys.length === 0) return;
    context.issues.push({ code: 'unrecognized_keys', keys, input: context.value, continue: true });
  });
}
