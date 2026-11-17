/**
 * The tool profiles of the core (Agent Plugins Extended §18.2.7; core §7.3.6, §7.3.7) as JavaScript
 * text, part of the program `run-core-handler.mjs`: the same table as `hooks/core/profiles.ts` and
 * `hooks/core/profile-results.ts`, which the adapter cannot import since it runs where the kit is not
 * installed. A hook never sees the facts only a client holds, so where the Claude Code fields do not
 * determine a value the profile is omitted (J18.16, J18.21): a shell command's exit code on
 * `PostToolUse` is `0` only without `returnCodeInterpretation`, an MCP tool's declared name is read
 * only when `__` occurs once after `mcp__`, an MCP call's completed profile is omitted (no
 * `CallToolResult`), and a text or notebook read gets no profile (the text Claude received is never
 * rebuilt from `file.content`).
 */
export const CORE_PROFILES_SOURCE = String.raw`const isObject = (value) => typeof value === 'object' && value !== null && !Array.isArray(value);
const record = (value) => (isObject(value) ? value : {});
const text = (value) => (typeof value === 'string' ? value : undefined);

const webUrl = (url) => {
  try {
    const { protocol } = new URL(url);
    return protocol === 'http:' || protocol === 'https:';
  } catch {
    return false;
  }
};

const modify = (kind, key, cwd, absolute) => (input) => {
  const written = text(input[key]);
  if (written === undefined) return undefined;
  const path = absolute && !isAbsolute(written) ? resolve(cwd, written) : written;
  return { name: 'file.modify', input: { operations: [{ kind, path }] } };
};

const shellProfile = (input) =>
  text(input.command) === undefined
    ? undefined
    : { name: 'shell', input: { command: input.command } };

const pendingProjectors = (cwd) => ({
  Bash: shellProfile,
  PowerShell: shellProfile,
  Monitor: shellProfile,
  Read: (input) =>
    text(input.file_path) === undefined
      ? undefined
      : { name: 'file.read', input: { path: input.file_path } },
  Edit: modify('edit', 'file_path', cwd, false),
  Write: modify('write', 'file_path', cwd, false),
  NotebookEdit: modify('edit', 'notebook_path', cwd, true),
  WebFetch: (input) =>
    text(input.url) !== undefined && webUrl(input.url)
      ? { name: 'web.fetch', input: { url: input.url } }
      : undefined,
  WebSearch: (input) =>
    text(input.query) === undefined
      ? undefined
      : { name: 'web.search', input: { queries: [input.query] } },
});

// The declared tool name: the text after the server segment of mcp__<server>__<tool>, only when the
// text after mcp__ holds __ once (a server segment or a declared name may hold __ itself).
function mcpToolName(toolName) {
  const rest = toolName.slice('mcp__'.length);
  const at = rest.indexOf('__');
  if (at !== rest.lastIndexOf('__')) return undefined;
  return at <= 0 || at + 2 >= rest.length ? undefined : rest.slice(at + 2);
}

function pendingProfile(toolName, toolInput, cwd) {
  if (toolName.startsWith('mcp__')) {
    const name = mcpToolName(toolName);
    return name === undefined
      ? undefined
      : { name: 'mcp', input: { name, arguments: record(toolInput) } };
  }
  const projectors = pendingProjectors(cwd);
  if (!isObject(toolInput) || !Object.hasOwn(projectors, toolName)) return undefined;
  return projectors[toolName](toolInput);
}

const EXIT_LINE = /^Exit code (\d+)\r?\n?/;
const joined = (parts) =>
  parts.filter((part) => typeof part === 'string' && part !== '').join('\n');

function shellResult(input, call) {
  if (call.ok) {
    const response = record(call.response);
    const background = input.run_in_background === true || response.backgroundTaskId !== undefined;
    if (background || response.interrupted === true) return undefined;
    // returnCodeInterpretation marks a non-error exit code with special meaning: the code is unknown.
    if (response.returnCodeInterpretation !== undefined) return undefined;
    return { output: joined([response.stdout, response.stderr]), exit_code: 0 };
  }
  if (input.run_in_background === true) return undefined;
  const line = EXIT_LINE.exec(call.error);
  if (line === null) return undefined;
  return { output: call.error.slice(line[0].length), exit_code: Number(line[1]) };
}

const binary = (mediaType, data) =>
  typeof mediaType === 'string' && typeof data === 'string'
    ? [{ type: 'binary', media_type: mediaType, data }]
    : undefined;

// A text or notebook read has no kind here: its content is the text Claude received, which a hook
// never sees, so the profile is omitted.
const READ_KINDS = {
  image: (file) => binary(file.type, file.base64),
  pdf: (file) => binary('application/pdf', file.base64),
  parts: (_file, response) =>
    Array.isArray(response.pages)
      ? response.pages.flatMap((page) => binary(record(page).mediaType, record(page).base64) ?? [])
      : undefined,
};

function readResult(_input, call) {
  if (!call.ok) return { error: call.error };
  const response = record(call.response);
  const kind = text(response.type);
  if (kind === undefined || !Object.hasOwn(READ_KINDS, kind)) return undefined;
  const content = READ_KINDS[kind](record(response.file), response);
  return content === undefined ? undefined : { content };
}

function fetchResult(_input, call) {
  if (!call.ok) return undefined;
  const response = record(call.response);
  const code = response.code;
  if (!Number.isInteger(code) || code < 100 || code > 599) return undefined;
  return { status_code: code, content: [{ type: 'text', text: text(response.result) ?? '' }] };
}

function searchItems(results) {
  if (!Array.isArray(results)) return [];
  return results.flatMap((entry) =>
    isObject(entry) && Array.isArray(entry.content)
      ? entry.content.flatMap((source) => {
          const { title, url, snippet } = record(source);
          if (typeof title !== 'string' || typeof url !== 'string' || url === '') return [];
          return [typeof snippet === 'string' ? { title, url, snippet } : { title, url }];
        })
      : [],
  );
}

// The mcp profile's result is the server's own CallToolResult, which a hook never receives.
const PROFILE_RESULTS = {
  shell: shellResult,
  'file.read': readResult,
  'file.modify': (_input, call) => (call.ok ? {} : { error: call.error }),
  'web.fetch': fetchResult,
  'web.search': (_input, call) =>
    call.ok ? { items: searchItems(record(call.response).results) } : { error: call.error },
  mcp: () => undefined,
};

function completedProfile(toolName, toolInput, call, cwd) {
  if (toolName === 'Monitor') return undefined;
  const pending = pendingProfile(toolName, toolInput, cwd);
  if (pending === undefined) return undefined;
  const result = PROFILE_RESULTS[pending.name](record(toolInput), call);
  return result === undefined ? undefined : { ...pending, result };
}
`;
