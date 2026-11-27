/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/mods_api_names.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('protocol/mods-api-names');

/**
 * The mods API by namespace and method name as the reference page lists it (plugins/mods/reference.md,
 * "Mods API methods"), in its order.
 */
export const MODS_API_REFERENCE: {
  readonly plugin: readonly ['name', 'root'];
  readonly ui: readonly [
    'resolve',
    'invalidate',
    'open',
    'close',
    'panes',
    'place',
    'focus',
    'scroll',
    'toast',
    'notify',
    'status',
    'log',
    'notice',
    'ask',
    'copy',
    'selection',
    'blit',
    'view',
  ];
  readonly theme: readonly ['set', 'clear'];
  readonly command: readonly ['register', 'run', 'list'];
  readonly tool: readonly ['register', 'call', 'check', 'list'];
  readonly agent: readonly ['register', 'spawn', 'list'];
  readonly model: readonly ['complete', 'fork', 'classify'];
  readonly prompt: readonly ['submit', 'read', 'fill', 'suggest', 'compose'];
  readonly turn: readonly ['abort'];
  readonly session: readonly [
    'messages',
    'cwd',
    'root',
    'model',
    'turns',
    'id',
    'repo',
    'surfaces',
    'usage',
    'version',
    'compact',
    'send',
    'append',
    'authorize',
  ];
  readonly config: readonly ['list', 'set'];
  readonly settings: readonly ['read'];
  readonly env: readonly ['get', 'set'];
  readonly fs: readonly ['read', 'write', 'list', 'exists', 'stat', 'ancestors'];
  readonly store: readonly ['get', 'set', 'delete', 'keys'];
  readonly state: readonly ['get', 'set'];
  readonly clock: readonly ['now', 'sleep', 'after', 'every'];
  readonly http: readonly ['fetch'];
  readonly process: readonly ['run', 'spawn'];
  readonly mcp: readonly ['call', 'connect'];
  readonly audio: readonly ['play', 'speak'];
  readonly telemetry: readonly ['log', 'mark'];
} = call('MODS_API_REFERENCE');

/**
 * The members of `$` the declarations name and the reference page does not (AGENTS.md, E2a: the
 * declarations win): `$.ui.cancelNotify` (claude-code/index.d.ts:2536), `$.prompt.media` (:3143) and
 * `$.prompt.paint` (:3168), `$.session.surface` (:2980) and `$.session.identity` (:3081).
 */
export const DECLARED_API_MEMBERS: {
  readonly ui: readonly ['cancelNotify'];
  readonly prompt: readonly ['media', 'paint'];
  readonly session: readonly ['surface', 'identity'];
} = call('DECLARED_API_MEMBERS');

/**
 * The mods API by namespace and method name: the `$` argument every hook receives, the reference's
 * members and then the declarations' others. Every method is also an event named
 * `<namespace>.<method>` (reference.md, "Mods API calls"), so this table is where the names of those
 * events come from.
 */
export const MODS_API: {
  readonly plugin: readonly ['name', 'root'];
  readonly ui: readonly [
    'resolve',
    'invalidate',
    'open',
    'close',
    'panes',
    'place',
    'focus',
    'scroll',
    'toast',
    'notify',
    'status',
    'log',
    'notice',
    'ask',
    'copy',
    'selection',
    'blit',
    'view',
    'cancelNotify',
  ];
  readonly theme: readonly ['set', 'clear'];
  readonly command: readonly ['register', 'run', 'list'];
  readonly tool: readonly ['register', 'call', 'check', 'list'];
  readonly agent: readonly ['register', 'spawn', 'list'];
  readonly model: readonly ['complete', 'fork', 'classify'];
  readonly prompt: readonly ['submit', 'read', 'fill', 'suggest', 'compose', 'media', 'paint'];
  readonly turn: readonly ['abort'];
  readonly session: readonly [
    'messages',
    'cwd',
    'root',
    'model',
    'turns',
    'id',
    'repo',
    'surfaces',
    'usage',
    'version',
    'compact',
    'send',
    'append',
    'authorize',
    'surface',
    'identity',
  ];
  readonly config: readonly ['list', 'set'];
  readonly settings: readonly ['read'];
  readonly env: readonly ['get', 'set'];
  readonly fs: readonly ['read', 'write', 'list', 'exists', 'stat', 'ancestors'];
  readonly store: readonly ['get', 'set', 'delete', 'keys'];
  readonly state: readonly ['get', 'set'];
  readonly clock: readonly ['now', 'sleep', 'after', 'every'];
  readonly http: readonly ['fetch'];
  readonly process: readonly ['run', 'spawn'];
  readonly mcp: readonly ['call', 'connect'];
  readonly audio: readonly ['play', 'speak'];
  readonly telemetry: readonly ['log', 'mark'];
} = call('MODS_API');

/**
 * The `status` of each entry `$.agent.list()` returns (reference.md, "Mods API methods"); `idle`
 * and `waiting` require Claude Code v2.1.289 or later, and the kit implements the current behaviour.
 */
export const AGENT_STATUSES: readonly [
  'pending',
  'running',
  'waiting',
  'idle',
  'completed',
  'failed',
  'killed',
] = call('AGENT_STATUSES');

export type AgentStatus = (typeof AGENT_STATUSES)[number];

export type ModsNamespace = keyof typeof MODS_API;

export type ModsMethod<N extends ModsNamespace> = (typeof MODS_API)[N][number];

/**
 * The calls whose time is the calling hook's own: "A `$` call's time never counts against the calling
 * hook's budget" (claude-code/index.d.ts:6589), but `$.clock.sleep`'s wait "is the hook's own time and
 * its budget runs on through it, as through no other `$` call" (:3641).
 */
export const OWN_TIME_CALLS: ReadonlySet<string> = call('OWN_TIME_CALLS');

/**
 * The calls declared to answer nothing (`=> void`, claude-code/index.d.ts:2400 `notice`, :2416
 * `invalidate`, :2470 `log`, :2503 `toast`, :2550 `status`): each returns at once and its event goes
 * on through the chain on its own.
 */
export const FIRED_CALLS: readonly [
  'ui.notice',
  'ui.invalidate',
  'ui.log',
  'ui.toast',
  'ui.status',
] = call('FIRED_CALLS');

export type FiredEventName = (typeof FIRED_CALLS)[number];

/**
 * The calls that answer at once, not with a promise: `$.ui.resolve`, the timers and the fired calls.
 * A misused one throws where it is called.
 */
export const AT_ONCE_CALLS: ReadonlySet<string> = call('AT_ONCE_CALLS');
