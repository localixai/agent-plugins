import { sideloadRefusal } from './sideload.ts';
import {
  refuse,
  type Contribution,
  type FlagContext,
  type McpConfigSource,
  type SessionPlan,
} from './types.ts';
import { wordsOf } from './values.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { positiveWholeEnv, wholeEnv } from '../../format/env-number.ts';

/**
 * `--mcp-config` and `--strict-mcp-config` (cli-reference.md, "CLI flags"; plugins/org.md,
 * `disableSideloadFlags`; headless.md, "Start faster with bare mode"). `--mcp-config` loads "MCP
 * servers from JSON files or strings"; with `-p` Claude Code "waits for still-pending servers to
 * connect before running the first turn, up to the `MCP_TIMEOUT` startup timeout, 30 seconds by
 * default". `--strict-mcp-config` means "only use MCP servers from `--mcp-config`, ignoring all other
 * MCP configurations". `disableSideloadFlags` rejects `--mcp-config` "unless every server in it is an
 * in-process `type: "sdk"` entry". What a server does once loaded is the MCP stream's; this maps the
 * flags to the sources and the wait.
 */

/** "30 seconds by default." */
export const DEFAULT_MCP_TIMEOUT_MS = 30_000;

const isInline = (value: string): boolean => value.trimStart().startsWith('{');

/**
 * A value that starts with `{` is the JSON itself and stays whole; any other holds one or more file
 * paths, "space-separated" (cli-reference.md, `--mcp-config`).
 */
const sourcesOf = (value: string): readonly McpConfigSource[] =>
  isInline(value)
    ? [{ kind: 'inline', text: value }]
    : wordsOf(value).map((path): McpConfigSource => ({ kind: 'file', path }));

type TextOf<K extends McpConfigSource['kind']> = (
  source: Extract<McpConfigSource, { kind: K }>,
  context: FlagContext,
) => string | undefined;

const TEXT_OF: { readonly [K in McpConfigSource['kind']]: TextOf<K> } = {
  inline: (source) => source.text,
  file: (source, context) => context.facts.files[source.path]?.text,
};

const textOf = <K extends McpConfigSource['kind']>(
  source: Extract<McpConfigSource, { kind: K }>,
  context: FlagContext,
): string | undefined => TEXT_OF[source.kind](source, context);

// oxlint-disable-next-line kit/no-kind-literal -- `type` is a field of the user's JSON, not a closed set of the kit
const isInProcessServer = (server: unknown): boolean => isRecord(server) && server.type === 'sdk';

/** The `mcpServers` object of a config, by name; a config that cannot be read has none. */
function serversIn(text: string | undefined): Readonly<Record<string, unknown>> | undefined {
  if (text === undefined) return undefined;
  try {
    const parsed: unknown = JSON.parse(text);
    const servers = isRecord(parsed) ? parsed.mcpServers : undefined;
    return isRecord(servers) ? servers : undefined;
  } catch {
    return undefined;
  }
}

/** Whether every server of the config is `type: "sdk"`; a config that cannot be read is not. */
function onlySdkServers(text: string | undefined): boolean {
  const servers = serversIn(text);
  return servers !== undefined && Object.values(servers).every(isInProcessServer);
}

/** The `type`s of a remote server: "a remote (`http` or `sse`) server" (mcp.md, "Server status detail"). */
const REMOTE_TYPES: ReadonlySet<unknown> = new Set(['http', 'sse']);
const isRemoteServer = (server: unknown): boolean =>
  isRecord(server) && REMOTE_TYPES.has(server.type);

/**
 * `MCP_TIMEOUT` is in milliseconds, a positive whole number read as every numeric variable is (spec
 * §3.6.1 item 2, J3.17: `2e3`, `64_000`); anything else is the default.
 */
const timeoutMs = (context: FlagContext): number =>
  positiveWholeEnv(context.facts.env.MCP_TIMEOUT) ?? DEFAULT_MCP_TIMEOUT_MS;

/**
 * "In a self-hosted environment, a shorter wait applies instead" (cli-reference.md, `--mcp-config`):
 * "the first turn waits up to 2 seconds for stdio servers that are still connecting", and
 * `CLAUDE_CODE_MCP_STARTUP_WAIT_MS` "changes only how long the wait lasts"
 * (self-hosted-environments-configuration.md, "Wait for MCP servers before the first turn").
 */
const SELF_HOSTED_MCP_WAIT_MS = 2_000;

function selfHostedWaitMs(context: FlagContext): number {
  // J3.17: a whole number (`2e3`, `64_000`); `0` skips the wait.
  return wholeEnv(context.facts.env.CLAUDE_CODE_MCP_STARTUP_WAIT_MS) ?? SELF_HOSTED_MCP_WAIT_MS;
}

const waitMs = (context: FlagContext): number =>
  context.facts.selfHosted === true ? selfHostedWaitMs(context) : timeoutMs(context);

/**
 * The servers the first turn does not wait for: "a server with a cached tool list skips the wait and
 * connects on first use" (only a remote one has a cached tool list: agent-sdk/mcp.md, "Connection
 * timing"), and in a self-hosted environment the wait covers stdio servers only, HTTP and SSE ones
 * connecting in the background.
 */
function waitSkips(sources: readonly McpConfigSource[], context: FlagContext): readonly string[] {
  const cached = new Set(context.facts.cachedToolLists ?? []);
  const selfHosted = context.facts.selfHosted === true;
  return sources.flatMap((source) =>
    Object.entries(serversIn(textOf(source, context)) ?? {})
      .filter(([name, server]) => isRemoteServer(server) && (selfHosted || cached.has(name)))
      .map(([name]) => name),
  );
}

export function mcpConfig(values: readonly string[], context: FlagContext): Contribution {
  const sources = values.flatMap(sourcesOf);
  const refusal = sideloadRefusal('--mcp-config', context.facts);
  if (
    refusal !== undefined &&
    !sources.every((source) => onlySdkServers(textOf(source, context)))
  ) {
    return refuse(refusal);
  }
  return {
    mcpConfigs: sources,
    ...(context.facts.print
      ? { mcpWaitMs: waitMs(context), mcpWaitSkips: waitSkips(sources, context) }
      : {}),
  };
}

export const strictMcpConfig = (): Contribution => ({ strictMcpConfig: true });

/** The places MCP servers are read from. */
export const MCP_READ_SOURCES = ['user', 'project', 'local', 'plugins', 'command-line'] as const;
export type McpReadSource = (typeof MCP_READ_SOURCES)[number];

/**
 * The sources a session reads servers from: `--strict-mcp-config` and `--bare` leave the command
 * line's alone ("only servers supplied on the command line connect"). How a managed MCP file
 * combines with `--strict-mcp-config` is the MCP stream's (managed-mcp.md).
 */
export function mcpSourcesRead(
  plan: Pick<SessionPlan, 'strictMcpConfig' | 'skipped'>,
): readonly McpReadSource[] {
  const commandLineOnly = plan.strictMcpConfig || plan.skipped.includes('mcp-servers');
  return commandLineOnly ? ['command-line'] : MCP_READ_SOURCES;
}
