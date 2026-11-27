import type { PluginIssue } from '../format/issues.ts';
import type { HttpClient } from '../ports/http-client.ts';
import type { ArchiveReader, ArchiveStats, Sha256 } from '../ports/marketplace.ts';
import type { ArchiveFetchPlan } from './types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Fetching an `archive` source (plugins/marketplace-reference.md, "archive plugin source";
 * plugins/host-marketplace.md, "Stay within the download limits for hosted files"): the download
 * goes through `HttpClient`, the digest through `Sha256`, the unzip is the host's (`ArchiveReader`
 * reports what the zip holds) and the kit judges every documented limit. A refusal is a
 * `PluginIssue`; nothing throws. `ArchiveReader.scan` lists the entries; the totals, the plugin root
 * and the paths and links that would leave the plugin directory are all judged here
 * (`archive-scan.ts`), before a host unzips anything.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/archive_source.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/archive-source');

export interface ArchivePorts {
  readonly http: HttpClient;
  readonly sha256: Sha256;
  readonly archive: ArchiveReader;
}

/** The verified zip and where the plugin root is in it: `''` at the top, or the one directory down. */
export interface FetchedArchive {
  readonly bytes: Uint8Array;
  readonly stats: ArchiveStats;
  readonly root: string;
}

/**
 * Downloads the archive and judges it; the verified zip and its plugin root, or the issue that
 * refuses it. The ports and the signal cross alone: a caller may pass a bigger object.
 */
export const fetchArchive = (
  plan: ArchiveFetchPlan,
  ports: ArchivePorts,
  { signal }: { readonly signal: AbortSignal },
): Promise<FetchedArchive | PluginIssue> =>
  call.async(
    'fetchArchive',
    plan,
    { http: ports.http, sha256: ports.sha256, archive: ports.archive },
    { signal },
  );
