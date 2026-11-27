import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceEntry, MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type {
  AcceptedCommand,
  HeaderLayer,
  HeaderOrigin,
  HeadersSetting,
  PluginOperation,
} from './header-policy.ts';
import type { HelperCache, HeadersHelperPorts } from './header-helper.ts';
import type { ResolvedHeaders } from './types.ts';

/**
 * The headers of a download (plugins/host-marketplace.md, "Authenticate archive downloads"; "Stay
 * within the download limits"; marketplace-reference.md, "Fields by type", "Plugin entries"):
 * `headers` and `headersHelper` can be set on the marketplace's `url` source and on an entry; the
 * entry's value replaces the source's of the same name, and within one place the command's output
 * replaces a header of the same name in `headers`. Redirects to another origin carry none of them:
 * `fetchArchive` drops them on that hop. The prompt that asks a user to accept an entry's command,
 * and the command's own execution, are the host's (`CommandRunner`); here the policy and the merge.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/download_headers.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/download-headers');

export interface DownloadHeaderPorts extends HeadersHelperPorts {
  /** One per host session, so a `url` source's command output is reused for 60 seconds. */
  readonly cache: HelperCache;
}

/** The `url` source of a marketplace, declared in a settings file. */
export type UrlSourceHeaders = Extract<MarketplaceSource, { kind: 'url' }> & {
  /** Unset on the first fetch after a user adds a marketplace by URL: that fetch supplies the name. */
  readonly name?: string | undefined;
};

export const urlSourceLayer = (source: UrlSourceHeaders, origin: HeaderOrigin): HeaderLayer =>
  call('urlSourceLayer', source, origin);

/** A plugin entry's fields (`MarketplaceEntry`, or an inline entry in settings). */
export type EntryHeaders = Pick<MarketplaceEntry, 'name' | 'headers' | 'headersHelper'>;

export interface EntryDeclaration {
  readonly origin: HeaderOrigin;
  readonly operation: PluginOperation;
  /** The command and archive URL the user accepted; unset when they accepted nothing. */
  readonly accepted?: AcceptedCommand | undefined;
}

export const entryLayer = (
  entry: EntryHeaders,
  archiveUrl: string,
  declared: EntryDeclaration,
): HeaderLayer => call('entryLayer', entry, archiveUrl, declared);

/**
 * The headers one place sends: its `headers`, with the command's output over them when the
 * command may run. Used for a `url` source's `marketplace.json` fetch and, through
 * `archiveDownloadHeaders`, for an archive. A command that fails, or a refused one that stops
 * the download, is a `PluginIssue`.
 */
export const resolveHeaders = (
  layer: HeaderLayer,
  setting: HeadersSetting,
  ports: DownloadHeaderPorts,
  { signal }: { readonly signal: AbortSignal },
): Promise<ResolvedHeaders | PluginIssue> =>
  call.async('resolveHeaders', layer, setting, ports, { signal });

export interface DownloadHeaderRequest {
  readonly archiveUrl: string;
  /** The marketplace's `url` source, when it was added by URL. */
  readonly marketplace?: HeaderLayer;
  readonly entry?: HeaderLayer;
}

/**
 * The headers of an archive download: the `url` source's when the archive is on its origin, then
 * the entry's over them. The result fills `ArchiveFetchPlan.headers` (`planPluginFetch` leaves it
 * empty because it knows no settings).
 */
export const archiveDownloadHeaders = (
  request: DownloadHeaderRequest,
  setting: HeadersSetting,
  ports: DownloadHeaderPorts,
  { signal }: { readonly signal: AbortSignal },
): Promise<ResolvedHeaders | PluginIssue> =>
  call.async('archiveDownloadHeaders', request, setting, ports, { signal });
