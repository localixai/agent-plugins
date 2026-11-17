/**
 * The settings oracle's cases where the kit is known to differ from the Agent SDK's resolver, by the
 * case's name, with why (a judgment, or the docs). `bun run oracle:settings` fails when a case so
 * marked agrees, so a fixed difference leaves this list.
 */
export const KNOWN: Readonly<Record<string, string>> = {
  'a key the project cannot set':
    'settings-reference.md scopes the key to user, local and managed; resolveSettings merges it raw',
  'defaultMode acceptEdits from project':
    'filterEscalatingDefaultMode drops acceptEdits from project settings; claude starts in acceptEdits, as permissions.md names only auto and bypassPermissions',
};
