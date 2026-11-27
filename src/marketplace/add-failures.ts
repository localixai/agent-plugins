import { coreModule } from '../wasm/bridge.ts';

/**
 * What `claude plugin marketplace add` and an install say when the source cannot be fetched
 * (plugins/troubleshooting.md, "Invalid git URL", "Path does not exist: <path>", "Marketplace file
 * not found at ...", "Git clone timed out after 120s", `Marketplace "<name>" not found`). Pure: the
 * host found the failure, the kit words it. The texts the docs quote are the docs'; the rest is the
 * output of `claude plugin marketplace add` 2.1.319, recorded in spec/judgments.md as `oracle 2.1.319`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/add_failures.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/add-failures');

export type AddFailure =
  /** A local path with nothing at it; the host resolved it against its working directory. */
  | { readonly kind: 'path-missing'; readonly path: string }
  /** A path to a file that is not a `.json` file. */
  | { readonly kind: 'not-json'; readonly path: string }
  /** The marketplace was cloned, found, or downloaded but holds no `marketplace.json` at `path`. */
  | { readonly kind: 'file-missing'; readonly path: string }
  /** A relative `source` whose directory is missing; `path` is "the absolute location Claude Code checked". */
  | { readonly kind: 'source-path-missing'; readonly path: string }
  /** The clone ran past `timeoutMs` (`CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS`, default 120000). */
  | { readonly kind: 'clone-timeout'; readonly timeoutMs: number };

/** The line a failed add ends with. */
export const addFailureMessage = (failure: AddFailure): string =>
  call('addFailureMessage', failure);

/**
 * "Claude Code checks every git address before running git": a protocol it doesn't support, and an
 * address git could read as naming a different server than the one shown, are refused with `Invalid
 * git URL` and the reason. The address itself is not echoed (the oracle prints `[redacted URL]`).
 */
export const gitUrlRefusal = (address: string): string | undefined =>
  call('gitUrlRefusal', address);

/** The accepted forms a bare name or hostname is told about (`Invalid marketplace source format`). */
export const ACCEPTED_SOURCE_FORMS: readonly string[] = call('ACCEPTED_SOURCE_FORMS');

export interface MissingMarketplace {
  readonly message: string;
  /** What to run next, as the docs give it; `undefined` when only the marketplace's owner can say. */
  readonly fix?: string;
}

/**
 * `Marketplace "<name>" not found` on `/plugin install <plugin>@<name>`: Claude Code has "no index to
 * look a marketplace name up in", so the fix is the marketplace's source, except for the official
 * marketplace and for a `claudeai-` name, which have a command.
 */
export const missingMarketplace = (name: string): MissingMarketplace =>
  call('missingMarketplace', name);

/**
 * Adding a marketplace whose `name` is not valid as the part after `@` in a plugin id is refused and
 * registers nothing; the text is the docs' ("Cannot add marketplace ..."). An empty name is left to the
 * reader's own "must have a name".
 */
export const idPartRefusal = (name: string): string | undefined => call('idPartRefusal', name);
