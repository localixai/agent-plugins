/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/mods_limits.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('protocol/mods-limits');

export const MOD_LIMITS: {
  readonly hookMs: 10000;
  readonly promptEditMs: 50;
  readonly catchMs: 1000;
  readonly lingerMs: 5000;
  readonly sessionEndMs: 1500;
  readonly runTimeoutMs: 30000;
  readonly runTimeoutMaxMs: 600000;
  readonly outputBytes: number;
  readonly maxTokens: 1024;
  readonly maxTokensCap: 64000;
  readonly fileBytes: number;
  readonly httpRequestChars: number;
  readonly httpResponseBytes: number;
  readonly httpMs: 30000;
  readonly redirects: 5;
  readonly reasonChars: 4096;
  readonly treeText: 100000;
  readonly fieldChars: 10000;
  readonly hrefChars: 2048;
  readonly treeDepth: 32;
  readonly treeNodes: 20000;
  readonly rasterCells: 262144;
  readonly imageBytes: number;
  readonly storeBytes: number;
  readonly messages: 4096;
  readonly toastMs: 4000;
  readonly invalidatePerSecond: 10;
  readonly invalidatePerSecondTerminal: 30;
  readonly paneColumns: 144;
  readonly paneColumnsAfterOpen: 110;
  readonly nameChars: 64;
  readonly placedRows: 500;
  readonly scopes: 2000;
  readonly nesting: 28000;
  readonly testMs: 5000;
} = call('MOD_LIMITS');

/** Command, tool, subagent type and pane names: letters, digits, `_` and `-`, up to `nameChars`. */
export const MOD_NAME_PATTERN: RegExp = call('MOD_NAME_PATTERN');
