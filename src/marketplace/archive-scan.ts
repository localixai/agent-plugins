/**
 * What the kit reads out of a zip's entry list without unzipping anything (plugins/host-
 * marketplace.md, "Stay within the download limits for hosted files"; plugins/marketplace-
 * reference.md, "archive plugin source"): the totals the extraction limits judge, and the names
 * and links that must not leave the plugin directory. Pure over `ArchiveReader.scan`.
 *
 * Judgments (spec/judgments.md): the docs name no zip-slip or link rule for an archive, so an entry
 * whose path would leave the folder it is extracted into, and a symbolic link whose target leaves
 * the plugin directory (followed through the other links in the zip), and an entry written through
 * a link, are refused whole (an archive has no marketplace to dereference into, and
 * "Outside the marketplace: the symlink is skipped for security" cannot skip an entry the host is
 * about to extract). A link is found by its name as a file system that folds names would compare
 * it, and a link that shares its name with another entry is refused: on such a file system `a/x`
 * is written through a link stored as `A`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/archive_scan.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { ArchiveScan, ArchiveStats } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/archive-scan');

/** The totals of a scan: derived, never reported by the host, so the limits cannot be argued with. */
export const archiveStats = (scan: ArchiveScan, zipBytes: number): ArchiveStats =>
  call('archiveStats', scan, zipBytes);

/**
 * The first entry that would be written outside the plugin directory, is written through a link,
 * shares its name with a link, or is a link that leaves it.
 */
export const unsafeEntryIssue = (scan: ArchiveScan, root: string): PluginIssue | undefined =>
  call('unsafeEntryIssue', scan, root);
