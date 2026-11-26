import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { SourceSite } from './plugin-source-fields.ts';

/**
 * An archive source's `url` "must use `https://` and can't point at a loopback, link-local, or
 * cloud-metadata host" (plugins/marketplace-reference.md, "archive plugin source"). The validator
 * reports a break on `source.url` with the text of spec §15.6.5 (oracle 2.1.325); a text that does
 * not parse as a URL draws `Invalid URL` first and the same report (J15.8). The scheme is read in
 * any letter case.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/archive_url.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/marketplace/archive-url');

export const archiveUrlIssues = (url: string, site: SourceSite): PluginIssue[] =>
  call('archiveUrlIssues', url, site);
