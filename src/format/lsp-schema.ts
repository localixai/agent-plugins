import { z } from 'zod';
import { closedObject } from './strict-keys.ts';

/**
 * The LSP server configuration as Claude Code's schema check reads it, one rule set for the loader
 * and the validator (plugins/manifest-reference.md, "lspServers": "Each server config is a strict
 * object"; spec §9.1.2, the oracle table of §9.1.3). The messages are the oracle's (2.1.325): zod's
 * own wording, and for `command` and `extensionToLanguage` the two texts it adds. A type error, a
 * fraction or a key without its dot aborts the entry, so inside the `lspServers` union it is a bare
 * `Invalid input` at the key; a too-small number, an empty map, a command with spaces and an
 * unknown key stay at their own path.
 */

/** `stdio` (default) or `socket`: "Claude Code accepts `socket` but runs every server over stdio". */
export const LSP_TRANSPORTS = ['stdio', 'socket'] as const;

const COMMAND_SPACES = 'Command should not contain spaces. Use args array for arguments.';
const NO_MAPPING = 'extensionToLanguage must have at least one mapping';

const text = z.string();
const strings = z.record(text, text);
const positive = z.number().int().positive();
const flag = z.boolean();

/** Judgment (spec §9.1.2): any whitespace counts as a space; a path that starts with `/` may hold them. */
const command = text
  .min(1)
  .refine((value) => value.startsWith('/') || !/\s/.test(value), { error: COMMAND_SPACES });

/** Every key starts with a dot (`.go`), and there is at least one. */
const extensions = z
  .record(text.regex(/^\./), text)
  .refine((map) => Object.keys(map).length > 0, { error: NO_MAPPING });

const lspServerSchema = closedObject({
  command,
  extensionToLanguage: extensions,
  args: z.array(text).exactOptional(),
  transport: z.enum(LSP_TRANSPORTS).exactOptional(),
  env: strings.exactOptional(),
  initializationOptions: z.unknown().exactOptional(),
  settings: z.unknown().exactOptional(),
  workspaceFolder: text.exactOptional(),
  startupTimeout: positive.exactOptional(),
  shutdownTimeout: positive.exactOptional(),
  requestTimeout: positive.exactOptional(),
  restartOnCrash: flag.exactOptional(),
  maxRestarts: z.number().int().min(0).exactOptional(),
  diagnostics: flag.exactOptional(),
});

/** A map of server name to configuration: `.lsp.json`, a named file, or the inline object. */
const lspMapSchema = z.record(text, lspServerSchema);

/**
 * The manifest's `lspServers`: a path, an inline map, or an array of either. A path is any string
 * here; its `./` and `.json` rules are the path checks' (validate/plugin-paths.ts).
 */
export const lspSourcesSchema = z.union([
  text,
  lspMapSchema,
  z.array(z.union([text, lspMapSchema])),
]);
