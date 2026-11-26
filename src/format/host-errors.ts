/**
 * The notices, warnings and errors Claude Code prints for the failures errors.md documents, each in
 * the wording the page quotes (docs/SPEC.md: message wording comes from the docs). Pure string
 * builders: the host decides when a failure happened and shows the text where the page says it
 * appears (conversation notice, stderr, debug log, `/status`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/host_errors.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/host-errors');

/** The startup notice for a skill, command or workflow whose name uses `anthropic-skills`. */
export const reservedSkillNameNotice = (input: {
  /** What to rename: a folder or file path, or the `name:` line to edit, as the first refused item shows it. */
  readonly rename: string;
  /** The reserved name the item uses: `anthropic-skills` or a name that starts with `anthropic-skills:`. */
  readonly name: string;
  /** How many more items were refused. */
  readonly more?: number;
}): string => call('reservedSkillNameNotice', input);

/** The startup notice when the agents' names and descriptions exceed 15,000 tokens. */
export const agentDescriptionsOverLimit = (estimatedTokens: number, folder: string): string =>
  call('agentDescriptionsOverLimit', estimatedTokens, folder);

/** The error of a command that finds `.mcp.json` is not a regular file or is over 2 MiB. */
export const mcpJsonUnreadable = (): string => call('mcpJsonUnreadable');

/** `claude mcp add` and `add-json` with `--scope managed`. */
export const cannotAddMcpToManagedScope = (): string => call('cannotAddMcpToManagedScope');

/** The reasons the page lists for organization settings that could not load, as it names them. */
export const organizationSettingsUnavailable = (reason: string, organization: string): string =>
  call('organizationSettingsUnavailable', reason, organization);

/** What the `Remote managed settings failed to load` warning says about the policy the session runs on. */
export type RemotePolicy = 'cached' | 'none';

/** The warning when server-managed settings could not be fetched or applied; `cause` names what failed. */
export const remoteManagedSettingsFailed = (cause: string, policy: RemotePolicy): string =>
  call('remoteManagedSettingsFailed', cause, policy);

/** The state lines of the fetch that appear in the Code tab. */
export const REMOTE_MANAGED_SETTINGS_STILL_LOADING: string = call(
  'REMOTE_MANAGED_SETTINGS_STILL_LOADING',
);
export const REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD: string = call(
  'REMOTE_MANAGED_SETTINGS_DID_NOT_LOAD',
);

/** The line when the settings loaded after one of the warnings; `applies` says whether a remote policy applies. */
export const remoteManagedSettingsLoaded = (applies: boolean): string =>
  call('remoteManagedSettingsLoaded', applies);

/** The error of a name that imitates an official marketplace, on every plugin it affects. */
export const marketplaceNameRefused = (name: string): string =>
  call('marketplaceNameRefused', name);

/** The marketplace's own error for an imitating name. */
export const MARKETPLACE_NAME_LOOKS_OFFICIAL: string = call('MARKETPLACE_NAME_LOOKS_OFFICIAL');

/** What `claude plugin marketplace add` says for an impersonating name. */
export const MARKETPLACE_NAME_IMPERSONATES: string = call('MARKETPLACE_NAME_IMPERSONATES');

/** The refusal to add a marketplace whose name is another spelling of a reserved name. */
export const reservedSpellingRefusal = (
  name: string,
  reserved: string,
  needsQuoting: boolean,
): string => call('reservedSpellingRefusal', name, reserved, needsQuoting);

/** The warning for a registered entry whose name is another spelling of a reserved name. */
export const reservedSpellingIgnored = (name: string, reserved: string): string =>
  call('reservedSpellingIgnored', name, reserved);

/** The stderr warning when denying the whole Bash tool leaves a Windows session with no shell tool. */
export const BASH_DENY_TURNS_OFF_POWERSHELL: string = call('BASH_DENY_TURNS_OFF_POWERSHELL');

/** The notification for an `asyncRewake` shell-form hook whose script path has a space and no quotes. */
export const hookNeedsQuotes = (event: string, script: string): string =>
  call('hookNeedsQuotes', event, script);

/** The startup banner row for a deny or ask rule of a file tool that file checks never consult. */
export const fileToolRuleBannerRow = (
  behavior: 'deny' | 'ask',
  rule: string,
  use: string,
): string => call('fileToolRuleBannerRow', behavior, rule, use);

/** The full warning for such a rule: `source` is a settings file path or the flag. */
export const fileToolRuleWarning = (input: {
  readonly behavior: 'allow' | 'deny' | 'ask';
  readonly source: string;
  readonly rule: string;
  /** `Edit` or `Read`, the rule kind to write instead. */
  readonly use: string;
  /** `Write(docs/**)` becomes `Edit(docs/**)`. */
  readonly replacement: string;
}): string => call('fileToolRuleWarning', input);

/** The startup banner row for a deny or ask rule whose tool name is unknown. */
export const matchesNoToolBannerRow = (behavior: 'deny' | 'ask', rule: string): string =>
  call('matchesNoToolBannerRow', behavior, rule);

/** The error when a `claude plugin` command cannot ask for approval of managed settings. */
export const MANAGED_SETTINGS_NEED_APPROVAL: string = call('MANAGED_SETTINGS_NEED_APPROVAL');

/** What Claude Code prints when the security approval dialog is declined. */
export const MANAGED_SETTINGS_NOT_APPROVED: string = call('MANAGED_SETTINGS_NOT_APPROVED');

/** The result an MCP tool call gets when the server is turned off. */
export const mcpServerDisabled = (name: string): string => call('mcpServerDisabled', name);

/** The warning for an MCP server whose name Claude Code reserves. */
export const mcpServerNameReserved = (name: string): string => call('mcpServerNameReserved', name);

/** The message for a Java MCP SDK server that rejects the elicitation capability. */
export const ELICITATION_REJECTED: string = call('ELICITATION_REJECTED');

/** The error result for a call to a tool that is not in the session's tool list; `hint` follows the name. */
export const noSuchTool = (name: string, hint?: string): string => call('noSuchTool', name, hint);

/** The code a path check reports: its own name, except those `REPORTED_AS` folds. */
export const reportedCheckCode = (code: string): string => call('reportedCheckCode', code);

/** `/plugin` Errors tab: a path the operating system could not check (any error but "not found"). */
export const pathCouldNotBeChecked = (component: string, path: string, code: string): string =>
  call('pathCouldNotBeChecked', component, path, code);

/** The same error as `claude plugin list` prints it. */
export const pathCouldNotBeCheckedInList = (
  component: string,
  path: string,
  code: string,
): string => call('pathCouldNotBeCheckedInList', component, path, code);

/**
 * `<component> path escapes plugin directory: <path>` as `claude plugin` commands print it:
 * `Path escapes plugin directory: ./../shared.md (commands)` (errors.md, "Path escapes plugin directory").
 */
export const pathEscapesInList = (component: string, path: string): string =>
  call('pathEscapesInList', component, path);

/** Where a marketplace plugin's manifest resolves when it is not inside the plugin or its marketplace. */
export const manifestNotRead = (manifest: string, reason: string): string =>
  call('manifestNotRead', manifest, reason);

/** The `/status` warning for `allowRead` or `allowedDomains` entries a managed lock ignores. */
export const sandboxAllowIgnored = (input: {
  readonly lock: 'read' | 'domains';
  readonly count: number;
  readonly file: string;
}): string => call('sandboxAllowIgnored', input);

/** The startup notice for skill or command names Claude Code refused; `names` is the count. */
export const skillNamesRefused = (count: number): string => call('skillNamesRefused', count);
