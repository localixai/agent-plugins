/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/web_fetch.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/web-fetch');

/** errors.md, "WebFetch cannot fetch localhost": the message of a refused URL, verbatim. */
export const LOCAL_HOST_MESSAGE: string = call('LOCAL_HOST_MESSAGE');

/** "WebFetch reads up to 100,000 characters of a page's content per call." */
export const FETCH_WINDOW_CHARACTERS: number = call('FETCH_WINDOW_CHARACTERS');

export interface CheckedUrl {
  /** The URL to fetch: `https` for an `http` one; `undefined` when it is refused. */
  readonly url: string | undefined;
  /** An `http` URL was changed to `https`. */
  readonly upgraded: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * What WebFetch does with a URL before it makes a request (tools-reference.md, "WebFetch tool
 * behavior"): it refuses `localhost` and any other hostname without a dot, with the message of
 * errors.md, and upgrades an `http` URL to `https`. Text that does not parse, or a scheme other
 * than http and https, is an "Invalid URL" issue (spec/judgments.md: errors.md names that message
 * for the refusals before v2.1.268, the docs say no more).
 */
export const checkWebFetchUrl = (text: string): CheckedUrl => call('checkWebFetchUrl', text);

export interface FetchedPart {
  /** Up to 100,000 characters of the page, from the offset. */
  readonly text: string;
  /** Characters of the page after this part. */
  readonly unread: number;
  /** The `offset` that reads the next part; `undefined` when nothing is unread. */
  readonly nextOffset: number | undefined;
  /** The note that tells Claude how much went unread; `undefined` when nothing did. */
  readonly notice: string | undefined;
}

/**
 * The part of a page WebFetch reads (tools-reference.md, "WebFetch tool behavior"; hooks.md,
 * `offset`: "number of characters to skip from the start of the page"): up to 100,000 characters
 * from `offset`, and how much went unread. Characters are code points, so a part never splits one;
 * an offset past the end reads nothing; an offset that is not a whole number counts as absent; the
 * wording of the notice is a judgment (spec/judgments.md).
 */
export const sliceFetched = (page: string, offset: number | undefined): FetchedPart =>
  call('sliceFetched', page, offset);
