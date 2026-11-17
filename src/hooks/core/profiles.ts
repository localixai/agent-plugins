import { isRecord } from '../../protocol/is-record.ts';
import type { CoreCallFacts, CoreToolProfile } from './types.ts';

/**
 * The portable view of a pending Claude Code call (Agent Plugins Extended §18.2.7; core §7.3.6): a
 * table by Claude Code tool name of 2.1.321, each entry deciding from the call's defined operation
 * and input whether its profile applies. A tool the table does not name gets none (`Glob`, `Grep`,
 * `Agent`, `Skill`, `AskUserQuestion` and every bookkeeping tool), and so does a call the entry's
 * criteria exclude (a `Monitor` with `ws`, a `WebFetch` of a relative URL).
 */

export interface ProfileContext {
  /** The session's working directory: a relative `notebook_path` is made absolute against it. */
  readonly cwd: string;
  readonly facts: CoreCallFacts;
}

type Input = Readonly<Record<string, unknown>>;
type Projector = (input: Input, context: ProfileContext) => CoreToolProfile | undefined;

const text = (input: Input, key: string): string | undefined => {
  const value = input[key];
  return typeof value === 'string' ? value : undefined;
};

const profile = (name: string, input: Input): CoreToolProfile => ({ name, input });

/** `shell`: the command string exactly, which the tool has the shell interpret. */
const shell: Projector = (input) => {
  const command = text(input, 'command');
  return command === undefined ? undefined : profile('shell', { command });
};

const fileRead: Projector = (input) => {
  const path = text(input, 'file_path');
  return path === undefined ? undefined : profile('file.read', { path });
};

/** `file.modify` with one operation of `kind` on the path the key names. */
const modify =
  (kind: 'edit' | 'write', key: string, absolute = false): Projector =>
  (input, context) => {
    const written = text(input, key);
    if (written === undefined) return undefined;
    const path = absolute && !written.startsWith('/') ? `${context.cwd}/${written}` : written;
    return profile('file.modify', { operations: [{ kind, path }] });
  };

const WEB_SCHEMES: ReadonlySet<string> = new Set(['http:', 'https:']);

/** `web.fetch` only for an absolute `http` or `https` URL (core §7.3.6, "Web fetch tool profile"). */
const webFetch: Projector = (input) => {
  const url = text(input, 'url');
  if (url === undefined || !URL.canParse(url)) return undefined;
  return WEB_SCHEMES.has(new URL(url).protocol) ? profile('web.fetch', { url }) : undefined;
};

const webSearch: Projector = (input) => {
  const query = text(input, 'query');
  return query === undefined ? undefined : profile('web.search', { queries: [query] });
};

const MCP_PREFIX = 'mcp__';

/**
 * The tool name an MCP call sends: the one the host gives (`CoreCallFacts.mcpToolName`), else the
 * text after the server segment of `mcp__<server>__<tool>` when that is the only reading: the text
 * after `mcp__` holds `__` once (J18.21). A server segment or a declared name may itself hold `__`
 * (§8.5), so any other full name gives no profile unless the host names the tool.
 */
function mcpToolName(toolName: string, facts: CoreCallFacts): string | undefined {
  if (facts.mcpToolName !== undefined) return facts.mcpToolName;
  const rest = toolName.slice(MCP_PREFIX.length);
  const at = rest.indexOf('__');
  if (at !== rest.lastIndexOf('__')) return undefined;
  return at <= 0 || at + 2 >= rest.length ? undefined : rest.slice(at + 2);
}

const PROJECTORS: Readonly<Record<string, Projector>> = {
  Bash: shell,
  PowerShell: shell,
  Monitor: shell,
  Read: fileRead,
  Edit: modify('edit', 'file_path'),
  Write: modify('write', 'file_path'),
  NotebookEdit: modify('edit', 'notebook_path', true),
  WebFetch: webFetch,
  WebSearch: webSearch,
};

/** The profile of a pending call, or `undefined` when none applies. */
export function pendingProfile(
  toolName: string,
  toolInput: unknown,
  context: ProfileContext,
): CoreToolProfile | undefined {
  const input = isRecord(toolInput) ? toolInput : {};
  if (toolName.startsWith(MCP_PREFIX)) {
    const name = mcpToolName(toolName, context.facts);
    return name === undefined ? undefined : profile('mcp', { name, arguments: input });
  }
  const projector = Object.hasOwn(PROJECTORS, toolName) ? PROJECTORS[toolName] : undefined;
  return isRecord(toolInput) ? projector?.(input, context) : undefined;
}
