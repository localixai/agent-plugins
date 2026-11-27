import type {
  BaseHookInput,
  ClientCapabilities,
  ElicitationHookInput,
  ElicitationHookSpecificOutput,
  ElicitationRequest,
  ElicitationResult,
  ElicitationResultHookSpecificOutput,
} from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ElicitationAnswer } from './mcp-elicitation-form.ts';

export {
  elicitationFieldProblems,
  elicitationFormFields,
  type ElicitationAnswer,
  type ElicitationField,
  type ElicitationFieldType,
  type ElicitationFormat,
  type ElicitationValue,
} from './mcp-elicitation-form.ts';

/**
 * MCP elicitation (mcp.md, "Respond to MCP elicitation requests", "Elicitation modes Claude Code
 * declares"): a server asks for input mid-task, in form mode (fields the server defines) or URL mode
 * (a link to open). Pure data: the dialog, the browser and the connection are the host's. The request,
 * the answer and the hook fields are the official types.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_elicitation.rs`, docs/RUST.md).
 */
const call = coreModule('format/mcp-elicitation');

export type ElicitationMode = 'form' | 'url';

/** What the client declares in `capabilities.elicitation`: both modes, or the bare `{}` (form only). */
export type ElicitationCapability = NonNullable<ClientCapabilities['elicitation']>;

export interface McpConnection {
  /** The MCP protocol revision the connection runs on, such as `2026-07-28`. */
  readonly protocolRevision: string;
  /** A stdio, sse, http or ws server of the user's Claude Code MCP configuration (not a claude.ai connector). */
  readonly userConfigured: boolean;
  /**
   * The server entry sets `"bareElicitationCapability": true` (errors.md, "MCP server rejected the
   * elicitation capability"): it is offered the bare `{}`, form mode only, on any revision.
   */
  readonly bareElicitationCapability?: boolean;
}

/** "Connections that use protocol revision 2026-07-28". */
export const NEW_REVISION: '2026-07-28' = call('NEW_REVISION');

/**
 * What `elicitation` is declared on a connection: `{form: {}, url: {}}` on revision 2026-07-28 and
 * later, and on earlier revisions to the servers of a user's configuration; the bare `{}` on other
 * connections of the earlier revisions, and to a server entry with `bareElicitationCapability: true`
 * (spec §17, "Elicitation capability rejected"). Revisions are dates, so they compare as text.
 */
export const declaredElicitation = (connection: McpConnection): ElicitationCapability =>
  call('declaredElicitation', connection);

/** The modes a server may request: "the bare `elicitation: {}` ... permits elicitation requests in form mode only". */
export const elicitationModes = (capability: ElicitationCapability): readonly ElicitationMode[] =>
  call('elicitationModes', capability);

/**
 * "Caps how long that argument can be": 8,000, "A URL with none of them reaches the cap at about 8,000
 * characters", and "every third character a `%` ... roughly 4,000".
 */
export const ELICITATION_URL_ARGUMENT_CAP: number = call('ELICITATION_URL_ARGUMENT_CAP');

/** "Its own character plus three escape characters": each such character counts four times. */
export const urlArgumentLength = (url: string): number => call('urlArgumentLength', url);

/** What the person can do with a URL request: over the cap "you can only decline the request". */
export const elicitationUrlChoices = (url: string): readonly ('open' | 'decline')[] =>
  call('elicitationUrlChoices', url);

/** What the person did with the dialog. */
export type ElicitationResponse =
  | { readonly action: 'accept'; readonly values: Readonly<Record<string, unknown>> }
  | { readonly action: 'decline' | 'cancel' };

/**
 * The answer to a request: a form's values checked against the fields its schema defines (judgment,
 * spec/judgments.md: the docs say the person "fills in the fields and submits", and do not say how the
 * client checks them; a value for a field the schema lacks is not sent), or the acceptance of a link;
 * declining or cancelling sends no content. A request in no mode is a form (the MCP default).
 */
export const answerElicitation = (
  request: ElicitationRequest,
  response: ElicitationResponse,
): ElicitationAnswer => call('answerElicitation', request, response);

/** What the `Elicitation` hook receives, in the official field names (hooks.md, "Elicitation"). */
export type ElicitationHookFacts = Omit<
  ElicitationHookInput,
  keyof BaseHookInput | 'hook_event_name'
>;

export const elicitationHookFacts = (request: ElicitationRequest): ElicitationHookFacts =>
  call('elicitationHookFacts', request);

/** What the session says about the dialog: with `permissions.denyInsteadOfPrompting` none is shown. */
export interface ElicitationSession {
  readonly denyInsteadOfPrompting?: boolean;
}

/**
 * "To auto-respond to elicitation requests without showing a dialog, use the `Elicitation` hook": an
 * output that names an `action` is the answer; one without is ignored (hooks.md), and the dialog shows
 * (`undefined`). With `permissions.denyInsteadOfPrompting` there is no dialog: a request no hook
 * answers is cancelled (settings-reference.md, "What else `denyInsteadOfPrompting` stops").
 */
export const autoRespondElicitation = (
  output: ElicitationHookSpecificOutput | undefined,
  session: ElicitationSession = {},
): ElicitationResult | undefined => call('autoRespondElicitation', output, session);

/** The `ElicitationResult` hook "can override the action or content before the response is sent". */
export const overrideElicitationResult = (
  result: ElicitationResult,
  output: ElicitationResultHookSpecificOutput,
): ElicitationResult => call('overrideElicitationResult', result, output);
