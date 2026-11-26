/**
 * The JavaScript built-ins the package's answers rest on, run here and recorded as vectors for
 * the crate's `js` module (docs/RUST.md, RS6): `JSON.parse` (its error text reaches a plugin's
 * issues: `Invalid JSON syntax: JSON Parse error: …`), `Number(text)`, `String(number)`,
 * `String.prototype.trim`, `new URL(text)` and `new URL(text, base)` (the members the package
 * reads of it), and `normalize`, `toUpperCase` and `toLowerCase` (an archive's names are folded
 * by them), `new RegExp(source, flags)` with `exec`, `replace`, `split` and `matchAll` (a hook's
 * matcher and a relevance signal are patterns from input), and npm's `semver`, which the
 * package's version ranges are read by. The inputs are fixed seeds and every single-character edit of them,
 * so the set is the same on every run and covers the edges a hand-picked list would miss; a
 * fixed list adds what one edit cannot reach (deep nesting, long numbers, every escape). The
 * runtime is the one Claude Code runs on (Bun: JavaScriptCore), whose wording the spec's oracle
 * recorded.
 *
 *   bun scripts/rust/js-oracle.ts     write crates/agent-plugins/tests/vectors/js/*.jsonl
 */
import { recordJsonParse } from './js-oracle/json.ts';
import { recordNormalize } from './js-oracle/normalize.ts';
import { recordNumbers } from './js-oracle/number.ts';
import { recordRegExps } from './js-oracle/reg-exp.ts';
import { recordRegExp } from './js-oracle/regexp.ts';
import { recordSemver } from './js-oracle/semver.ts';
import { recordUrlBases, recordUrls } from './js-oracle/url.ts';

recordJsonParse();
recordNumbers();
recordRegExps();
recordNormalize();
recordUrls();
recordUrlBases();
recordRegExp();
recordSemver();
