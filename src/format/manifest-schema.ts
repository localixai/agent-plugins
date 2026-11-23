import { z } from 'zod';
import { USER_CONFIG_KEY } from './user-config-refs.ts';
import { optionValue, optionsProblems } from './user-config-options.ts';
import { lspSourcesSchema } from './lsp-schema.ts';
import { monitorSourcesSchema } from './monitor-schema.ts';

/**
 * The top-level keys of `.claude-plugin/plugin.json` and their documented types
 * (plugins/manifest-reference.md, "Fields"). Strict where the docs say so ("Unrecognized fields"):
 * `userConfig` options, `channels` entries, inline LSP configurations and monitor entries. Inline
 * hook and MCP configs are checked by their own readers, not here.
 */

const path = z.string();
const pathOrPaths = z.union([path, z.array(path)]);
const object = z.record(z.string(), z.unknown());
const fileOrInline = z.union([path, object, z.array(z.union([path, object]))]);
/**
 * The inline LSP map and monitors array are strict objects of the manifest (spec §2.1 item 4,
 * §9.1.1 item 3, §9.2.1): an unknown key or a bad value in one keeps the plugin from loading.
 */
const monitors = monitorSourcesSchema;

const OPTION_TYPES = ['string', 'number', 'boolean', 'directory', 'file'] as const;

const userConfigFields = z.strictObject({
  type: z.enum(OPTION_TYPES),
  title: z.string(),
  description: z.string(),
  required: z.boolean().optional(),
  default: z.union([z.string(), z.number(), z.boolean(), z.array(z.string())]).optional(),
  options: z.array(optionValue).optional(),
  multiple: z.boolean().optional(),
  sensitive: z.boolean().optional(),
  min: z.number().optional(),
  max: z.number().optional(),
});

/** An option with the `options` rules, each problem at its own path (`options`, `default`). */
export const userConfigOptionSchema = userConfigFields.superRefine((option, context) => {
  for (const problem of optionsProblems(option)) {
    context.addIssue({ code: 'custom', path: [problem.path], message: problem.message });
  }
});
export type UserConfigOption = z.infer<typeof userConfigOptionSchema>;

const userConfig = z.record(z.string().regex(USER_CONFIG_KEY), userConfigOptionSchema);

/** The manifest says an entry that sets both or neither "fails validation". */
export const COMMAND_ENTRY_ONE_OF = 'Set exactly one of "source" or "content"';

const commandEntrySchema = z
  .object({
    source: path.optional(),
    content: z.string().optional(),
    description: z.string().optional(),
    argumentHint: z.string().optional(),
    model: z.string().optional(),
    allowedTools: z.array(z.string()).optional(),
  })
  .refine((entry) => (entry.source === undefined) !== (entry.content === undefined), {
    message: COMMAND_ENTRY_ONE_OF,
  });
export type CommandEntry = z.infer<typeof commandEntrySchema>;

/**
 * A dependency string has at most one `@`: `"a@b@c"` fails the manifest as `Invalid input`
 * (spec J16.17, oracle 2.1.325). Shared with the validator's schema.
 */
export const dependencyText = z
  .string()
  .refine((text) => text.indexOf('@') === text.lastIndexOf('@'));

const dependency = z.union([
  dependencyText.min(1),
  z.object({
    name: z.string().min(1),
    marketplace: z.string().optional(),
    version: z.string().optional(),
  }),
]);
export type PluginDependency = z.infer<typeof dependency>;

const channel = z.strictObject({
  server: z.string(),
  displayName: z.string().optional(),
  userConfig: userConfig.optional(),
});
/** A `channels` entry: a channel bound to one of the plugin's MCP servers. */
export type PluginChannel = z.infer<typeof channel>;

const url = z.string().refine((value) => URL.canParse(value), { message: 'Must parse as a URL' });

export const MANIFEST_SCHEMAS = {
  $schema: z.string(),
  name: z.string(),
  displayName: z.string(),
  version: z.string(),
  description: z.string(),
  author: z.object({ name: z.string(), email: z.string().optional(), url: z.string().optional() }),
  homepage: url,
  repository: z.string(),
  license: z.string(),
  keywords: z.array(z.string()),
  metadata: object,
  icon: z.string(),
  documentationUrl: z.string(),
  supportUrl: z.string(),
  privacyPolicyUrl: z.string(),
  termsOfServiceUrl: z.string(),
  defaultEnabled: z.boolean(),
  dependencies: z.array(dependency),
  settings: object,
  // Read by `format/engines.ts` and `format/network.ts`: a value that is not an object is ignored.
  engines: object,
  network: object,
  userConfig,
  types: path,
  channels: z.array(channel),
  skills: pathOrPaths,
  commands: z.union([pathOrPaths, z.record(z.string(), commandEntrySchema)]),
  agents: pathOrPaths,
  hooks: fileOrInline,
  mcpServers: fileOrInline,
  lspServers: lspSourcesSchema,
  outputStyles: pathOrPaths,
  workflows: pathOrPaths,
  experimental: z.object({
    themes: pathOrPaths.optional(),
    monitors: monitors.optional(),
    evals: pathOrPaths.optional(),
  }),
  // Moved under `experimental`; still loaded at the top level, with a validator warning.
  themes: pathOrPaths,
  monitors,
} as const;

export type ManifestKey = keyof typeof MANIFEST_SCHEMAS;

/** The whole manifest: `name` required, every other documented key optional. */
export const manifestSchema = z.object(MANIFEST_SCHEMAS).partial().required({ name: true });

/** A manifest that loaded. */
export type PluginManifest = Readonly<z.output<typeof manifestSchema>>;

export const isManifestKey = (key: string): key is ManifestKey =>
  Object.hasOwn(MANIFEST_SCHEMAS, key);
