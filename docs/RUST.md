# The Rust core

`crates/agent-plugins` is the kit's implementation: the specification at the pinned revision
(`spec/SOURCE.json`), with the package's modules, in Rust. A Rust host (llx) uses the crate
directly. Every other language goes through one boundary: `agent_plugins::registry`, each export
by name (`format/path-text#isWithin`), JSON in and out. `crates/agent-plugins-wasm` is that
boundary compiled to WebAssembly, and the npm package is its thin wrapper: a module of `src/`
keeps its TypeScript signatures, types and doc comments, and its functions call the core through
`src/wasm/bridge.ts`. Another language (Python, Go) gets the same module through its WebAssembly
runtime and a wrapper of its own; nothing in the core is specific to JavaScript but `js.rs`, the
JavaScript semantics the answers must keep.

The package's TypeScript is being moved into the core module by module, bottom layer first. A
module is in one of five states:

1. **TypeScript**: not ported yet; the package runs it as before.
2. **Ported**: its Rust file starts with ``//! Port of `src/<module>.ts` at sha256:<hash>.``,
   declares its exports (`exports!`, below), and is held to the vectors recorded from the
   TypeScript (below). The package still runs the TypeScript.
3. **Switched**: the TypeScript file is a wrapper over the core (`coreModule`), the vectors are gone,
   and the package's own tests of the module, and of everything above it, run on the Rust. Its
   header says ``//! Port of `src/<module>.ts`, switched.``
4. **Stays TypeScript**, by its nature (below): the core may keep its own counterpart, whose
   header says ``//! The core's counterpart of `src/<module>.ts`, which stays TypeScript: …`` and
   which exports nothing.
5. **Core only**: no TypeScript calls it any more, because every module that did now runs in the
   core. Its wrapper is deleted, it exports nothing, and its header says ``//! Ported from
   `src/<module>.ts`; only the core uses it now, so it has no wrapper.`` An export no TypeScript
   calls is removed the same way, from the wrapper and from `exports!` (D1): the boundary carries
   only what the package uses.

### What stays TypeScript

A function stays in the host's language when crossing the boundary would cost more than its work
or cannot be done: the boundary copies every argument as JSON, so a guard over a host value
(`protocol/is-record.ts`: one `typeof`) would copy the whole value at every node of a walk, and a
value with a cycle cannot be copied at all; and a pass over host text of any size that the host's
runtime does natively (`format/utf8.ts`: `TextEncoder` over a command's output, up to 64 MiB)
would copy the text there and back for a few operations a byte. The rule: a function that only
looks at the outside of a host value, holds host objects by identity, or makes one native pass
over host data of unbounded size, stays TypeScript; one that computes an answer from data
crosses. Once its callers are in the core, such a module is core-only: the text arrives through a
port once and stays there. The ports (`src/ports/`) are the host's types and stay TypeScript too.

## The boundary

- Each ported module ends with `crate::registry::exports! { "<module>" { "<export>" => ok
  |param: Type, …| body; … } }` listing every name its TypeScript exports at run time, in order;
  `registry::MODULES` lists the modules. How a result is answered is `ok` (`None` is
  `undefined`), `nullable` (`None` is `null`) or `fallible` (`Err` is the thrown message).
  Missing arguments are `undefined` and extra ones ignored, as in JavaScript.
- An answer is `{"result"}`, `{"throws"}` (the wrapper throws an `Error` with the message) or
  `{"fault"}` (a bug: an unknown name, arguments that do not decode).
- Values are JavaScript's, `js::JsValue` in the core: `undefined` and the numbers JSON has no
  text for cross tagged, at every depth and both ways, `{"$undefined": true}` and `{"$number":
  "NaN" | "Infinity" | "-Infinity" | "-0"}` (`src/wasm/wire.ts`, `registry/wire.rs`, the
  recorder's spelling); a `Set`, `Map`, `Uint8Array` and `RegExp` as `registry::Set`, `Map`,
  `Bytes` and `RegExp`. The core writes its answer's text itself, each number as JavaScript
  writes it (`1`, `1e+21`), so its JSON is `JSON.stringify`'s but for the tags.
- `bun run build:wasm` (`scripts/wasm/build.ts`) builds the module and writes it as base64 into
  `module.ts` (in `src/wasm/`) (not committed), so the package still reads nothing of the machine it runs
  on (L3). Run it before the package's tests, type check or build.

### Effects: an export that waits on the host

An export that takes a port, a callback or an `AbortSignal`, or that the TypeScript declares
`async`, runs in the core too, and the core still does no I/O and calls nothing across the
boundary: the host drives it with a plain loop, so any language can, and a Rust host implements
its ports without an async runtime.

1. `registry::start(name, args)` runs the export until it is done or waits, and answers a step:
   `{"done": answer}`, or `{"task": t, "needs": [{"id", "port", "method", "args"}]}`. A need is
   a method of a value the host keeps (`port`, its number), named and called as the TypeScript
   port's (`files.readText(path, signal)` is `readText` with `[path, signal]`); a callback's
   method is `call`.
2. The host does each need and gives its answer to `registry::resume(t, {"answers": [{"id",
   "result"} | {"id", "throws"}], "aborted": [n]})`, which runs the task on to its next step.
   Every need asked in one poll comes in one step, so a `join` in the core is concurrent work on
   the host, as `Promise.all` was. A host that throws ends the task with `{"rethrow": id}`: the
   call throws the very value the host threw, as an `await` in the TypeScript did (a port's own
   failure is a `PortFailure` in its result, `src/ports/failure.ts`, never a throw).
3. What the host keeps crosses as its number, at any depth, both ways (`registry/remote.rs`,
   `src/wasm/wire.ts`): a port (an object with methods) `{"$port": n}`, a function `{"$fn": n}`,
   an `AbortSignal` `{"$signal": n, "aborted": bool}`. The signal's flag in the core is set when
   a resume lists it in `aborted`: the core never waits on an abort, the host's port watches its
   own signal. A signal the core makes itself crosses as `{"$signal": null, "aborted": bool}`.

The core keeps each waiting task's future in a table of its thread and polls it with a waker that
does nothing (`registry/task.rs`); a port method of the host's writes its need and is pending
until the answer is in. In `exports!`, `async` before the kind marks such an export, its
parameters `registry::Port`, `registry::Callback`, `ports::Signal` or data, and its body may
`.await`:

```rust
"readJson" => async ok |files: Port, path: String, signal: Option<Signal>|
    read_json(&files, &path, signal.as_ref()).await;
```

Its wrapper is `call.async('readJson', files, path, signal)`, which returns a promise; a
function the TypeScript declares synchronous keeps `call('f', …)`, and a callback among its
arguments is answered as the core asks (a callback that returns a promise there is refused).
`registry::call` answers such an export with a fault. A Rust host passes its own port values and
closures, each answering with a ready future, and takes the answer with `ports::now(future)`.

### Handles: a result with methods

A result the TypeScript returns as an object with methods (a hook engine, a plugin host) is a
`registry::Handle` in the core, declared with `methods!` as a module declares its exports, each
method sync or `async`:

```rust
crate::registry::methods! {
    Counter {
        "add" => ok |this, by: f64| this.add(by);
        "addLater" => async ok |this, by: Callback| this.add(by.call::<f64>(vec![]).await);
    }
}
```

It crosses as `{"$handle": h, "sync": ["add"], "async": ["addLater"]}`; the core keeps the object
under `h`. The host calls a method by the name `<h>#<method>` through `start` and `resume` (a sync
one through `call` too), and `registry::release(h)` forgets it. The wrapper builds an object with
those methods (`src/wasm/drive.ts`) and releases the handle when the object is collected
(`FinalizationRegistry`); a method's call reads the host values of the call that made the handle,
so a port the core keeps in it is still the host's. `registry/probe` holds an export of each kind
for the loop's tests (`tests/wasm/loop.test.ts`); no module of `src/` names it.

## Switching a module

1. The module is ported and its exports are held to every recorded call
   (`cargo test --test differential`).
2. Rewrite `src/<module>.ts` as its wrapper: the same exports, types and doc comments, each
   function calling the core through `const call = coreModule('<module>')`, as `call('<export>',
   …args)` (`call.async` for one that returns a promise, "Effects" above); the module comment
   names the Rust file. Types stay TypeScript until they are generated from the core.
3. `bun run build:wasm`, then the module's tests and the tests of the modules that import it.
4. Its header becomes ``//! Port of `src/<module>.ts`, switched.`` and its vectors are deleted:
   the package's tests hold it now. `bun scripts/rust/ports.ts --check` checks that a switched
   module's wrapper calls exactly the exports the Rust declares.

## How a ported module is held to the TypeScript

Five checks, each deterministic:

1. **Vectors.** `bun scripts/rust/record.ts` runs the package's own tests with every exported
   function of `src/` wrapped (`scripts/rust/recorder/`) and writes each distinct call whose
   arguments are plain data to `crates/agent-plugins/tests/vectors/<module>.jsonl`: the arguments,
   what the function returned or threw, and the requirement ids of the `spec(...)` blocks the
   calling tests ran under. `crates/agent-plugins/tests/differential/` holds every ported Rust function to every
   recorded call of its namesake: 100%, or the test fails. With no paths it records every test
   and writes every file anew; with test files, it adds what they record (never a folder: that is
   a whole suite, a heavy run).
2. **The oracle.** `spec/oracle/` holds what Claude Code itself did with fixture plugins (synced
   from the spec, never edited). Each case runs through the crate's loading and must give the
   recorded outcome.
3. **The corpus.** The spec's corpus of real plugins with `claude plugin validate`'s verdicts
   (`extend/.cache/oracle/` of the spec checkout, local only). The crate's validator gives each
   plugin the recorded verdict.

4. **The built-ins.** Where an answer rests on JavaScript itself (`JSON.parse`'s error text, which
   reaches a plugin's issues; `Number(text)`; a number's text; `trim`; `new URL(text)`, read by the `url` crate and corrected where
   JavaScriptCore differs), `crates/agent-plugins/src/js.rs`
   is held to the built-ins as the runtime Claude Code runs on (Bun) answers them:
   `bun scripts/rust/js-oracle.ts` runs them on fixed seeds and every one-character edit of them
   and writes `crates/agent-plugins/tests/vectors/js/`. Never write such a rule from memory: add its inputs to the
   oracle.
5. **Currency.** Every port's first line names its TypeScript source and the source's hash
   (``//! Port of `src/format/messages.ts` at sha256:0a336bd48f23.``), and the vectors carry the
   spec revision and the source hashes they were recorded at (`crates/agent-plugins/tests/vectors/SOURCES.json`).
   `bun scripts/rust/ports.ts --check` fails on a port whose source changed, on vectors recorded
   from an older source, and on vectors recorded at another spec revision than `spec/SOURCE.json`
   pins. So a spec change cannot reach the package without the crate's checks going red.

`bun scripts/rust/coverage.ts` writes `crates/agent-plugins/COVERAGE.md`: every module of `src/`
in its state (switched, ported, stays TypeScript, TypeScript), read from the Rust headers.

The recorded vectors are a local cache, not committed (`.gitignore`): they hold a ported module
only until it is switched, and `bun scripts/rust/record.ts` writes them again in about a minute.
The JavaScript built-ins' oracle, `crates/agent-plugins/tests/vectors/js/`, is the crate's own
test data and is committed. A port's interface in `src/ports/` and any other module that stays
TypeScript keep a hash in their counterpart's header, so `ports.ts --check` still says when the
TypeScript they follow changed.

## Rules

| Id  | Rule |
| --- | --- |
| RS1 | A module of `src/` is a module of the crate, file for file: `src/format/code-points.ts` is `crates/agent-plugins/src/format/code_points.rs`, and a folder such as `src/format/` is `crates/agent-plugins/src/format.rs` beside its folder. Layers are the package's (docs/ARCHITECTURE.md): a module uses only its own folder and lower layers. |
| RS2 | An exported function keeps its name in snake case and answers what its namesake answers. Every export is declared in its module's `exports!` block under its TypeScript name, and `crates/agent-plugins/tests/differential/registry.rs` holds every declared export to every recorded call, called by name as the wrapper calls it. A call with no Rust spelling (an `undefined` where the parameter is no `JsValue` or `Option`) is counted and left out; nothing else is. |
| RS3 | No I/O, no ambient state: files, commands, requests, the environment and the clock are ports (`crates/agent-plugins/src/ports/`), traits whose methods return a `BoxFuture`. The crate never spawns, sleeps or blocks (clippy.toml's disallowed list). |
| RS4 | No panic: no `unwrap`, `expect`, `panic!`, indexing that can fail, or slicing inside a character. A plugin's mistake is a `PluginIssue`, as in the package. |
| RS5 | The lints in `Cargo.toml` are hard. An exception is an `#[expect(lint, reason = "...")]` on the smallest item. At most 300 lines of code per file, 100 per function; `x.rs` beside `x/`, never `mod.rs`. |
| RS6 | JavaScript's semantics where they reach a result live in `crates/agent-plugins/src/js.rs`: UTF-16 length and order (`code_unit_length`, `compare_code_units`), `trim`'s blanks, a number's text. A regular expression keeps JavaScript's classes: `\d`, `\w` and `\b` are ASCII there (spell them `[0-9]`, `[A-Za-z0-9_]` or use `(?-u:...)`), `\s` is `js::is_whitespace`'s set. |
| RS7 | Shapes are the package's JSON: an interface is a struct with `#[serde(rename_all = "camelCase")]`, an optional member an `Option` that is skipped when `None`, a union of strings an enum renamed to its strings, a tagged union an enum with `#[serde(tag = "...")]`. A value the package types `unknown`, or reads from JSON or YAML, is a `js::JsValue` (`undefined`, `NaN`, the infinities and `-0` included; `JsValue::Undefined` is a missing argument); a `number` field that may hold one of those four is serialized with `js::value::serialize_number`. Member order is the package's: a `JsObject` keeps order, maps are `IndexMap`. The library depends on no JSON library: `clippy.toml` disallows `serde_json::Value` and `serde_json::from_str`, which only the differential tests use, to read the vectors. |
| RS9 | A port mirrors its source so a TypeScript diff applies to it at a glance: the same functions in the same order, the same names, tables copied verbatim in the same order, the spec citations of the doc comments kept. Its first line is its port header, stamped by `bun scripts/rust/ports.ts --stamp <file>` when the port matches its source; `ports.ts --check` keeps it honest. |
| RS8 | A dependency earns its place, with its reason in a comment beside it in `Cargo.toml`: `serde`, `indexmap`, `regex` and `url` (the WHATWG URL parser, so no URL parser of our own is kept current) so far; `serde_json` for the tests only. |

## Where the crate answers differently

Only where Rust cannot hold what JavaScript holds, each a decision written here and in the
module's doc comment, never a silent difference:

- **JSON nested past 1000 levels is refused** (`JSON Parse error: Nesting deeper than 1000
  levels`), where JavaScriptCore reads any depth: a `JsValue` that deep overflows the
  stack when it is dropped or printed, which would end the host's process. No plugin file comes
  near it.
- **A lone surrogate** (`"\ud800"`) reads as U+FFFD: a Rust string holds only whole characters.
  The vectors leave such cases out.
- **A lookup by a name from input reads own keys only.** Where the TypeScript indexes a table
  with a name it was given (`VARIANTS[transport][field]` in `format/core/mcp-server`,
  `ENTRY_COMMANDS[type]` in `format/mcpb-manifest`, a saved setting by its key), a name like
  `constructor`, `toString` or `__proto__` reached `Object.prototype` and answered a function;
  the core answers "not there", which is what the tables mean.

## When the spec moves

The spec moves first, the package follows, then the crate (AGENTS.md, E7). `bun run spec:behind`
says when the pin is behind; catching up is one change:

1. `bun run spec:sync main`, then the TypeScript: tests and code until `bun run check` passes.
2. `bun scripts/rust/record.ts` (a heavy run: the whole suite). The vectors now carry the new
   spec revision and source hashes.
3. `bun scripts/rust/ports.ts --check` lists what is stale: each port whose source changed. For
   each, read `git diff <old>..HEAD -- <source>` (the hash in the header finds the revision the
   port was made from: `git log --format=%h -- <source>` and compare `ports.ts`'s hash of each),
   apply the same change to the port, and `--stamp` it.
4. `cargo test --test differential` holds every ported module to the new vectors, and the
   package's tests (`bun run check`) hold every switched one; a module of `src/` not moved yet
   shows in `COVERAGE.md`, never as a failure.
5. `crates/agent-plugins/scripts/check`, then both halves are committed together.

## Commands

| Task | Command |
| --- | --- |
| Record every vector (a heavy run: the whole suite, about a minute) | `bun scripts/rust/record.ts` |
| Record what some test files call | `bun scripts/rust/record.ts tests/format/skill-name.test.ts …` |
| The differential tests | `cargo test --test differential` |
| Every check of the crate | `crates/agent-plugins/scripts/check` |
| The coverage report | `bun scripts/rust/coverage.ts` |
| Record the built-ins' oracle | `bun scripts/rust/js-oracle.ts` |
| Which ports are stale | `bun scripts/rust/ports.ts` (`--check` fails on one) |
| Stamp a port after it matches its source | `bun scripts/rust/ports.ts --stamp <file>…` |
| Is the spec ahead of the pin | `bun run spec:behind` |
