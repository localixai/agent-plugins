import type { PluginIssue } from '../issues.ts';
import { isBlockedHost } from './blocked-hosts.ts';
import { fieldError, type SourceSite } from './plugin-source-fields.ts';

/**
 * An archive source's `url` "must use `https://` and can't point at a loopback, link-local, or
 * cloud-metadata host" (plugins/marketplace-reference.md, "archive plugin source"). The validator
 * reports a break on `source.url` with the text of spec §15.6.5 (oracle 2.1.325); a text that does
 * not parse as a URL draws `Invalid URL` first and the same report (J15.8). The scheme is read in
 * any letter case.
 */
const ARCHIVE_URL =
  'Archive URLs must use https:// and must not point at a loopback, link-local, or cloud-metadata host';

const hostOf = (link: string): string | undefined => {
  try {
    return new URL(link).hostname;
  } catch {
    return undefined;
  }
};

export function archiveUrlIssues(url: string, site: SourceSite): PluginIssue[] {
  const host = hostOf(url);
  const allowed = host !== undefined && /^https:\/\//i.test(url) && !isBlockedHost(host);
  if (allowed) return [];
  const unparsed = host === undefined ? [fieldError(site, 'url', 'Invalid URL')] : [];
  return [...unparsed, fieldError(site, 'url', ARCHIVE_URL)];
}
