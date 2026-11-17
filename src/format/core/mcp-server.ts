import { isRecord } from '../../protocol/is-record.ts';
import { defined } from '../defined.ts';
import { commandProblem } from './command.ts';
import { extensionsProblem, implementedExtensions, type CoreExtensions } from './extensions.ts';
import { CORE_VARIABLES, resolveCwd, SYMBOLIC_PATHS } from './placeholders.ts';
import { headersProblem, urlProblem } from './remote.ts';

/**
 * One server of a core `mcp.json` (core §7.2.1): an object with a `type` that matches exactly one of
 * three closed variants. An unknown field, an unknown `type` or a field of another variant makes the
 * entry invalid; each variant takes an optional `extensions` (core §8.1). The rules are the schema's
 * (`schemas/1.1.0/mcp.schema.json`, `#/$defs/server`) with the text's: the command token, the `cwd`
 * forms and containment, the reserved `env` names (core §9.2), the URL and header rules.
 */

export const CORE_TRANSPORTS = ['stdio', 'streamable-http', 'sse'] as const;
export type CoreTransport = (typeof CORE_TRANSPORTS)[number];

export interface CoreStdioServer {
  readonly type: 'stdio';
  readonly command: string;
  readonly args?: readonly string[];
  readonly env?: Readonly<Record<string, string>>;
  readonly cwd?: string;
  readonly extensions?: CoreExtensions;
}

export interface CoreRemoteServer {
  readonly type: 'streamable-http' | 'sse';
  readonly url: string;
  readonly headers?: Readonly<Record<string, string>>;
  readonly extensions?: CoreExtensions;
}

export type CoreMcpServer = CoreStdioServer | CoreRemoteServer;

type Check = (value: unknown) => string | undefined;

const strings: Check = (value) =>
  Array.isArray(value) && value.every((one) => typeof one === 'string')
    ? undefined
    : 'must be an array of strings';

function envProblem(value: unknown): string | undefined {
  if (!isRecord(value)) return 'must be an object of strings';
  const reserved = CORE_VARIABLES.find((name) => Object.hasOwn(value, name));
  if (reserved !== undefined) return `must not set ${reserved}: the client supplies it`;
  const bad = Object.entries(value).find(([, member]) => typeof member !== 'string');
  return bad === undefined ? undefined : `"${bad[0]}" must be a string`;
}

function cwdProblem(value: unknown): string | undefined {
  if (typeof value !== 'string') return 'must be a string';
  const resolved = resolveCwd(value, SYMBOLIC_PATHS);
  return 'problem' in resolved ? resolved.problem : undefined;
}

const typeCheck =
  (transport: CoreTransport): Check =>
  (value) =>
    value === transport ? undefined : `must be "${transport}"`;

const remoteFields = (transport: CoreTransport): Readonly<Record<string, Check>> => ({
  type: typeCheck(transport),
  url: urlProblem,
  headers: headersProblem,
  extensions: extensionsProblem,
});

/** The closed fields of each variant and their checks: a registry over the transports. */
const VARIANTS: { readonly [T in CoreTransport]: Readonly<Record<string, Check>> } = {
  stdio: {
    type: typeCheck('stdio'),
    command: commandProblem,
    args: strings,
    env: envProblem,
    cwd: cwdProblem,
    extensions: extensionsProblem,
  },
  'streamable-http': remoteFields('streamable-http'),
  sse: remoteFields('sse'),
};

const REQUIRED: { readonly [T in CoreTransport]: readonly string[] } = {
  stdio: ['type', 'command'],
  'streamable-http': ['type', 'url'],
  sse: ['type', 'url'],
};

const isTransport = (value: unknown): value is CoreTransport =>
  CORE_TRANSPORTS.some((known) => known === value);

const otherVariantHas = (field: string): boolean =>
  CORE_TRANSPORTS.some((transport) => Object.hasOwn(VARIANTS[transport], field));

function fieldProblem(transport: CoreTransport, field: string, value: unknown): string | undefined {
  const check = VARIANTS[transport][field];
  if (check !== undefined) {
    const problem = check(value);
    return problem === undefined ? undefined : `"${field}" ${problem}`;
  }
  return otherVariantHas(field)
    ? `"${field}" belongs to another transport than "${transport}"`
    : `unknown field "${field}"`;
}

/** The entry as a server, or why it is invalid (core §7.2.2 item 3: the server is skipped). */
export function readCoreServer(
  raw: unknown,
  implemented: readonly string[],
): { readonly server: CoreMcpServer } | { readonly problem: string } {
  if (!isRecord(raw)) return { problem: 'a server configuration must be an object' };
  if (raw.type === undefined) return { problem: 'a server configuration must have a "type"' };
  if (!isTransport(raw.type)) return { problem: `unknown "type" ${JSON.stringify(raw.type)}` };
  const transport = raw.type;
  const missing = REQUIRED[transport].find((field) => raw[field] === undefined);
  if (missing !== undefined) return { problem: `"${missing}" is required for "${transport}"` };
  for (const [field, value] of Object.entries(raw)) {
    const problem = fieldProblem(transport, field, value);
    if (problem !== undefined) return { problem };
  }
  return { server: BUILD[transport](raw, implementedExtensions(raw.extensions, implemented)) };
}

const text = (value: unknown): string => (typeof value === 'string' ? value : '');

const stringList = (value: unknown): readonly string[] | undefined =>
  Array.isArray(value) ? value.filter((one): one is string => typeof one === 'string') : undefined;

function stringMap(value: unknown): Readonly<Record<string, string>> | undefined {
  if (!isRecord(value)) return undefined;
  return Object.fromEntries(
    Object.entries(value).filter(
      (entry): entry is [string, string] => typeof entry[1] === 'string',
    ),
  );
}

type Raw = Readonly<Record<string, unknown>>;
type Build = (raw: Raw, extensions: CoreExtensions | undefined) => CoreMcpServer;

const remote =
  (type: 'streamable-http' | 'sse'): Build =>
  (raw, extensions) => ({
    type,
    url: text(raw.url),
    ...defined({ headers: stringMap(raw.headers), extensions }),
  });

/** The typed server of each variant, built once its fields passed their checks. */
const BUILD: { readonly [T in CoreTransport]: Build } = {
  stdio: (raw, extensions) => ({
    type: 'stdio',
    command: text(raw.command),
    ...defined({
      args: stringList(raw.args),
      env: stringMap(raw.env),
      cwd: typeof raw.cwd === 'string' ? raw.cwd : undefined,
      extensions,
    }),
  }),
  'streamable-http': remote('streamable-http'),
  sse: remote('sse'),
};
