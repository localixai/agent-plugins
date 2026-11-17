import { isRecord } from '../../protocol/is-record.ts';
import { pendingProfile, type ProfileContext } from './profiles.ts';
import type { CoreToolProfile } from './types.ts';

/**
 * The portable view of a completed call (Agent Plugins Extended §18.2.7; core §7.3.7): the pending
 * profile of the call as it ran, with the result projected from Claude Code's `tool_response` on
 * `PostToolUse` or its `error` on `PostToolUseFailure`. Where the result cannot be given (no exit
 * code, no HTTP status, no `CallToolResult`, no text Claude received), the whole profile is omitted,
 * as core §7.3.7 requires (Judgments J18.16, J18.21).
 */

/** What the call came to: its response, or the failure text Claude received. */
export type Completed =
  | { readonly ok: true; readonly response: unknown }
  | { readonly ok: false; readonly error: string };

type Result = Readonly<Record<string, unknown>> | undefined;
type Projector = (
  input: Readonly<Record<string, unknown>>,
  call: Completed,
  context: ProfileContext,
) => Result;

const record = (value: unknown): Readonly<Record<string, unknown>> =>
  isRecord(value) ? value : {};

const text = (value: unknown): string | undefined =>
  typeof value === 'string' ? value : undefined;

const EXIT_LINE = /^Exit code (\d+)\r?\n?/;

/** A command with no exit code: run in the background or interrupted (Judgment J18.16). */
const noExitCode = (
  input: Readonly<Record<string, unknown>>,
  response: Readonly<Record<string, unknown>>,
) =>
  input.run_in_background === true ||
  response.backgroundTaskId !== undefined ||
  response.interrupted === true;

const joined = (parts: readonly (string | undefined)[]): string =>
  parts.filter((part): part is string => part !== undefined && part !== '').join('\n');

/**
 * J18.21: the exit code of a completed command when the host gives none. `PostToolUse` means the
 * command succeeded, so `0`, except where `returnCodeInterpretation` marks a non-error exit code
 * with special meaning: that code is unknown, and the profile is omitted.
 */
const successCode = (response: Readonly<Record<string, unknown>>): number | undefined =>
  response.returnCodeInterpretation === undefined ? 0 : undefined;

/** `shell`: the output Claude received and the exit code the host observed. */
const shell: Projector = (input, call, context) => {
  if (call.ok) {
    const response = record(call.response);
    if (noExitCode(input, response)) return undefined;
    const exit = context.facts.exitCode ?? successCode(response);
    if (exit === undefined) return undefined;
    const output = joined([text(response.stdout), text(response.stderr)]);
    return { output, exit_code: exit };
  }
  if (input.run_in_background === true) return undefined;
  const line = EXIT_LINE.exec(call.error);
  const exit = context.facts.exitCode ?? (line === null ? undefined : Number(line[1]));
  if (exit === undefined) return undefined;
  const output = line === null ? call.error : call.error.slice(line[0].length);
  return { output, exit_code: exit };
};

const binary = (mediaType: unknown, data: unknown) =>
  typeof mediaType === 'string' && typeof data === 'string'
    ? [{ type: 'binary', media_type: mediaType, data }]
    : undefined;

type ReadKind = (
  file: Readonly<Record<string, unknown>>,
  response: Readonly<Record<string, unknown>>,
  context: ProfileContext,
) => readonly unknown[] | undefined;

/**
 * The text Claude received from the call, which only the host has: byte for byte the call's
 * `tool_result` string (J18.16). The docs fix no format for its line numbers, so it is never rebuilt
 * from `file.content`; without it the profile is omitted.
 */
const receivedText: ReadKind = (_file, _response, context) =>
  context.facts.readText === undefined
    ? undefined
    : [{ type: 'text', text: context.facts.readText }];

/** One content list per `FileReadOutput` variant (sdk-tools.d.ts 0.3.321). */
const READ_KINDS: Readonly<Record<string, ReadKind>> = {
  text: receivedText,
  image: (file) => binary(file.type, file.base64),
  pdf: (file) => binary('application/pdf', file.base64),
  parts: (_file, response) =>
    Array.isArray(response.pages)
      ? response.pages.flatMap((page) => binary(record(page).mediaType, record(page).base64) ?? [])
      : undefined,
  notebook: receivedText,
};

const fileRead: Projector = (_input, call, context) => {
  if (!call.ok) return { error: call.error };
  const response = record(call.response);
  const kind = text(response.type);
  const read = kind !== undefined && Object.hasOwn(READ_KINDS, kind) ? READ_KINDS[kind] : undefined;
  const content = read?.(record(response.file), response, context);
  return content === undefined ? undefined : { content };
};

const fileModify: Projector = (_input, call) => (call.ok ? {} : { error: call.error });

/** `web.fetch`: only a fetch with an HTTP response has a profile. */
const webFetch: Projector = (_input, call) => {
  if (!call.ok) return undefined;
  const response = record(call.response);
  const { code } = response;
  if (typeof code !== 'number' || !Number.isInteger(code) || code < 100 || code > 599)
    return undefined;
  return { status_code: code, content: [{ type: 'text', text: text(response.result) ?? '' }] };
};

/** One `{ title, url, snippet? }` per source; a string entry is commentary and gives none. */
function searchItems(results: unknown): readonly unknown[] {
  if (!Array.isArray(results)) return [];
  return results.flatMap((entry) =>
    isRecord(entry) && Array.isArray(entry.content)
      ? entry.content.flatMap((source) => {
          const { title, url, snippet } = record(source);
          if (typeof title !== 'string' || typeof url !== 'string' || url === '') return [];
          return [{ title, url, ...(typeof snippet === 'string' ? { snippet } : {}) }];
        })
      : [],
  );
}

const webSearch: Projector = (_input, call) =>
  call.ok ? { items: searchItems(record(call.response).results) } : { error: call.error };

/** `mcp`: the server's own `CallToolResult`, which only the host has (`McpOutput` is processed). */
const mcp: Projector = (_input, _call, context) => context.facts.mcpResult;

const PROJECTORS: Readonly<Record<string, Projector>> = {
  shell,
  'file.read': fileRead,
  'file.modify': fileModify,
  'web.fetch': webFetch,
  'web.search': webSearch,
  mcp,
};

/** Tools whose pending profile has no completed form: every `Monitor` result. */
const NO_RESULT: ReadonlySet<string> = new Set(['Monitor']);

/** The profile of a completed call, or `undefined` when none applies or its result is unknown. */
export function completedProfile(
  toolName: string,
  toolInput: unknown,
  call: Completed,
  context: ProfileContext,
): CoreToolProfile | undefined {
  if (NO_RESULT.has(toolName)) return undefined;
  const pending = pendingProfile(toolName, toolInput, context);
  if (pending === undefined) return undefined;
  const project = Object.hasOwn(PROJECTORS, pending.name) ? PROJECTORS[pending.name] : undefined;
  const result = project?.(record(toolInput), call, context);
  return result === undefined ? undefined : { ...pending, result };
}
