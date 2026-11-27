/** Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/url_pattern.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/hooks/url-pattern');

export const matchUrlPattern = (pattern: string, url: string): boolean =>
  call('matchUrlPattern', pattern, url);
