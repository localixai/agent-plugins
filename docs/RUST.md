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
  Missing arguments are `undefined` and extra ones ignored, as in JavaScript. A rest parameter
  (`...events: HookEvent[]`) is a last parameter `events: registry::Rest<String>`, the call's
  arguments from its place on, each decoded (`"eventSet" => ok |events: Rest<String>|
  event_set(events.0);`); its wrapper spreads them: `export const eventSet = (...events:
  readonly HookEvent[]): ReadonlySet<HookEvent> => call('eventSet', ...events)`.
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
   "result"} | {"id", "throws", "name"?}], "aborted": [n]})`, which runs the task on to its next
   step (`throws` is `cause instanceof Error ? cause.message : String(cause)`, `name` an
   `Error`'s `name`).
   Every need asked in one poll comes in one step, so a `join` in the core is concurrent work on
   the host, as `Promise.all` was. A host that throws ends the task with `{"rethrow": id}`: the
   call throws the very value the host threw, as an `await` in the TypeScript did (a port's own
   failure is a `PortFailure` in its result, `src/ports/failure.ts`, never a throw), unless the
   core catches it (item 6).
3. What the host keeps crosses as its number, at any depth, both ways (`registry/remote.rs`,
   `src/wasm/wire.ts`): a port (an object with methods) `{"$port": n}`, a function `{"$fn": n}`,
   an `AbortSignal` `{"$signal": n, "aborted": bool}`, a promise `{"$promise": …}` (item 7). The
   signal's flag in the core is set when a resume lists it in `aborted`: the core never waits on
   an abort, the host's port watches its own signal. A signal the core makes itself crosses as
   `{"$signal": null, "aborted": bool}`.
4. A property is read by the need `.<name>` with no arguments (`Port::member`): a port's own
   (`ScanFolder.dir`, `.files`), or a member of a record that crossed as a port because one of
   its members is a function (a `WorktreeLocation` with its `isSymlink`); a function read that
   way is a `Callback`, an absent member `undefined`. A parameter that may be either takes both
   shapes, `#[serde(untagged)] enum { Host(Port), Data(T) }` (`hooks/output/worktree_path.rs`).
   An object the TypeScript types as an interface of its own module rather than a port of
   `crate::ports` (a policy, a catalog, a token counter) is called by `Port::method(name, args)`,
   its method with `this` the object, and its module implements its trait for `Port` with that
   (`agents/preload_skills.rs`).
5. A function of the core's that the host calls later (a timer's `fire`) crosses as `{"$call":
   h}`, a `registry::Function`: a handle (below) whose one sync method is `call`, which the host
   makes a function.
6. A `try { await port.method(…) } catch (cause) { … }` is `registry::catch_host(future).await`,
   `Ok` with the future's output or `Err(registry::Thrown { message, name, need })` with the
   host's throw (`registry/catch.rs`). The scope that catches is the one awaiting the need when
   its answer comes, as the `try` awaiting a promise in JavaScript, wherever the call was made;
   the innermost catches, and on a throw the scope drops its future, so a need it was still
   waiting on is never asked (asked in the same poll) or its late answer is dropped. A catch that
   throws again (`if (signal?.aborted === true) throw cause`) writes `return
   cause.rethrow().await;` (a future that never completes, so it is any type): the scope around
   it catches it, or the task ends with `{"rethrow": need}` and the call throws the very value.
   The host keeps what it threw by need until the call ends. A Rust host's ports never throw, so
   for it `catch_host` answers `Ok`. `foundation/folder_read.rs` is the pattern:

   ```rust
   let read = catch_host(async { folder.files().await.read_text(path, signal).await }).await;
   match read {
       Ok(text) => TextRead { text, issue: None },
       Err(cause) => {
           if signal.is_some_and(Signal::is_aborted) {
               return cause.rethrow().await;
           }
           let reason = cause.message;
           // … the warning
       }
   }
   ```

7. A promise crosses as a value, at any depth, both ways (`registry/promise.rs`):
   - **The host's promise** (a `HookOutcome.background` given to `settleDeliveries`) crosses as
     `{"$promise": n}` and is a `registry::HostPromise<T>` in the core: a future whose first poll
     asks the need `then` of value `n` with no arguments, which the host answers when the promise
     settles, with its value, or with a throw for a rejection, so `catch_host` catches it as any
     need's. Given back, it crosses as `{"$promise": n}` again and is the host's own promise (a
     `JsValue` that holds the tag gives it back the same way).

     ```rust
     "settleDeliveries" => async ok |background: Vec<HostPromise<BackgroundDelivery>>|
         settle_deliveries(background).await;
     // in the body: match catch_host(promise).await { Ok(delivery) => …, Err(_) => … }
     ```

     Its wrapper is the plain one, `call.async('settleDeliveries', background)`; a sync
     `call('f', …)` given a promise refuses it when the core awaits it, as for any need.
   - **A promise of the core's** (`startInBackground`'s `done`) is a `registry::CorePromise<T>`,
     made from any `Send` future with `CorePromise::new(async move { … })`; it is a future a Rust
     host awaits. In an answer it crosses as `{"$promise": {"task": t}}`: the core keeps the
     future as task `t`, not polled yet, and the host's first `resume(t)` with no answers polls
     it. The wrapper makes it a JavaScript `Promise` that drives task `t` with the loop of
     `call.async` (`src/wasm/drive.ts`), its needs answered with the host values of the call that
     answered it (as a handle's), and that settles with the task's answer: its value, or the very
     value a host's throw it did not catch was. The task ends when it settles, or when the host
     cancels it (`registry::cancel`): the wrapper does when the loop is collected, as it can
     never go on (it waits on a host promise nothing holds). Make it from an `async` block or an
     `async fn`'s call, which asks nothing until polled: a port method already called has asked in
     the task that called it, and awaiting it in another task is a fault.

     ```rust
     pub struct BackgroundRun { pub done: CorePromise<BackgroundResult> } // serde tag "kind"
     let done = CorePromise::new(async move { settle(&runner, request).await … });
     "startInBackground" => ok |runner: Port, request: CommandRun, mode: Mode|
         start_in_background(runner, request, mode);
     ```

     Its wrapper keeps the TypeScript's kind: `call('startInBackground', runner, request, mode)`
     for a sync function, `call.async` for an `async` one; the `Promise` comes out of the answer.
   - **Order.** What JavaScript's microtask order gave the package's tests, and the bridge keeps:
     the core's promise runs at once to its first wait when the answer that holds it is read, so
     the host work it starts (`runner.run`) has started before the call returns, as an `async`
     function runs to its first `await` when called; it settles only after every host promise it
     awaits has, and never before the call that made it has answered, so the answer that holds it
     resolves first unless its host work is already done; two core promises are two tasks and two
     loops, each settling when its own needs do, in no order between them; the host's promises
     are awaited in one step, so `then` of each is asked together, as `Promise.all` did, and the
     results keep the order given. The number of microtask turns between a host promise settling
     and the core's answer is not kept: no test may count them.

`registry::Port` implements every port trait of `crates/agent-plugins/src/ports/`, one file of
`registry/remote/` each, each method the need of its TypeScript namesake with its arguments, the
answer read as the Rust type: `T | PortFailure` is a `Result`, an answer the type cannot hold a
fault. A method the TypeScript port answers at once (`Environment.get`, `Locations.homeDir`,
`Clock.now`) returns a future in the Rust trait too, as the host answers it as a need; a Rust host
returns a ready one.

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
`registry::call` answers such an export when it asks the host nothing (a token counter left to
the kit's own), and with a fault when it would wait. A Rust host passes its own port values and
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

A method the TypeScript declares synchronous whose body asks the host (a `Locations` that
delegates to the host's, `loading/resolve/without_plugin.rs`) is `"homeDir" => sync async ok
|this| …;`: listed as sync, so the wrapper's object answers at once, and run as an async body
whose needs the host answers as they come, with the ports of the call that made the handle.

It crosses as `{"$handle": h, "sync": ["add"], "async": ["addLater"]}`; the core keeps the object
under `h`. The host calls a method by the name `<h>#<method>` through `start` and `resume` (a sync
one through `call` too), and `registry::release(h)` forgets it. The wrapper builds an object with
those methods (`src/wasm/drive.ts`) and releases the handle when the object is collected
(`FinalizationRegistry`); a method's call reads the host values of the call that made the handle,
so a port the core keeps in it is still the host's. The object given back to the core, as an
argument at any depth, crosses as `{"$handle": h}` (`src/wasm/wire.ts` knows the objects it made)
and is read as `registry::Own<T>`, the core's own object, neither copied nor asked: a parameter
`core: Own<SessionCore>` is the session the host holds. `registry/probe` holds an export of each kind
for the loop's tests (`tests/wasm/loop.test.ts`); no module of `src/` names it.

### The host session: one handle, ids instead of identity

A session keeps what outlives a turn (`src/host/session-core.ts`, docs/ARCHITECTURE.md, `host`),
and the TypeScript told its parts apart by identity: a `WeakMap` per session or state, a `WeakSet`
of `once` claims, `!==` on host states, a registration compared with `===`. Identity does not
cross the boundary, so the session's state is one core handle, `SessionCore`
(`crates/agent-plugins/src/host/session_core.rs`), and the core tells its parts apart by numbers
it hands out:

- **The registry** is the core's: each `RegisteredHandler` it holds has a serial, given when it
  is registered (a hook registered again under the same id is a new serial, as it was a new
  object). The rules of `session-registry` (what a state declares, what an update keeps, what
  `addHooks` replaces), the dynamic watch list and the repeats of an async `SessionStart` are the
  core's too. Each module keeps its file and its exports, which take the session's handle back
  (`registry::Own<SessionCore>`, "Handles" above): `session_registry.rs`'s `registerEntries`,
  `session_hook_scope.rs`'s `addHooks`, `recordOutcome`, `hasHooks`. The background queue's
  deliveries are a handle of their own (`createBackgroundQueue`), as its tests make one alone.
- **A registration** is its prefix and a serial: a `HookHandle` is a handle of the core whose
  `remove` removes it only while the prefix still holds that serial (the `===` of
  `registerEntries`), and a turn scope is a handle that keeps its registrations and removes them
  at `end`.
- **A state** is taken up once per host object: the edge compares the state object it last gave
  with `cell.current()` (`currentOf`) and, when it changed, hands the core the facts it reads
  (`takeUp`: each active plugin's id, name and hooks; the gates' `settingsHooks` and
  `pluginOnlyHooks`), one crossing per update, none per use.
- **What stays TypeScript, at the edge** (`src/host/session-core.ts`): what is the host's. The
  state cell and the last state object; the host's closures (`onBackground`'s listeners, kept in
  a `Set` and removed by identity: the core answers a delivery with what to announce, the edge
  calls them); the host's promises (`.then` of each background promise reports `settle` into the
  core by value, `.finally` drops the `once` registrations it spent). The registry crosses to
  the TypeScript dispatch as a mirror the edge keeps by serial: `entries` answers the serials in
  order and only the entries this edge has not been sent yet, so an entry keeps its object for its
  life (a `once` claim, `Selection.monitor` and a test's `toBe` see the same one) and an unchanged
  registry crosses as numbers.
- **Not moved yet, each with its module**: the `once` claims (`hooks/select/once`: by serial in the
  core, the TypeScript dispatch mapping its entries to serials through the mirror until
  `run-all`, `dispatch` and `engine` run in the core); the skill-hook ledger (with
  `session-skill-hooks`, which fills it); the monitor ledger (with `session-monitors`; a
  subagent's core then shares its parent's ledger as one `Arc`, and `LEDGERS` goes); the engine
  cache, the Stop cap and the environment files (`session-environment`'s `WeakMap`, a field of
  the core) move with `engine`, whose dispatch becomes a method of the session, its stop cap a
  flag instead of the host object `UNCOUNTED`. `session-skill-entries`' memo by `HostState`
  becomes the core's by the state it took up, its entries naming their plugin by index into the
  state's `active` list, which the edge maps back to the host's `ActivePlugin` and `readBody`.

One crossing per operation, never per field: an `addHooks` is one call (register, filter by the
gates, replace the prefix, keep it in the turn scope), a `recordOutcome` one (watch paths, the
repeat check, answering the promises to watch), and a read of an unchanged registry answers
`null`.

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

A module whose exported zod schema (a value with methods; `crates/agent-plugins/src/zod.rs` builds
it in Rust) another TypeScript module imports is ported but not switched until its importers move:
a schema cannot cross the boundary, so the importer would lose it.

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
   reaches a plugin's issues; `Number(text)`; a number's text; `trim`; `new URL(text)` and
   `new URL(text, base)`, read by the `url` crate and corrected where JavaScriptCore differs;
   `normalize`, `toUpperCase` and `toLowerCase`; `new RegExp(source, flags)` with `exec`,
   `test`, `replace`, `split` and `matchAll`, run by the `regress` crate and respelled where
   JavaScriptCore reads a pattern otherwise, `js::regexp`), `crates/agent-plugins/src/js.rs`
   is held to the built-ins as the runtime Claude Code runs on (Bun) answers them:
   `bun scripts/rust/js-oracle.ts` runs them on fixed seeds and every one-character edit of them
   and writes `crates/agent-plugins/tests/vectors/js/`. Never write such a rule from memory: add its inputs to the
   oracle.
5. **Currency.** Every port's first line names its TypeScript source and the source's hash
   (``//! Port of `src/format/issues.ts` at sha256:<hash>.``), and the vectors carry the
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
| RS1 | A module of `src/` is a module of the crate, file for file: `crates/agent-plugins/src/format/code_points.rs` is `crates/agent-plugins/src/format/code_points.rs`, and a folder such as `src/format/` is `crates/agent-plugins/src/format.rs` beside its folder. Layers are the package's (docs/ARCHITECTURE.md): a module uses only its own folder and lower layers. |
| RS2 | An exported function keeps its name in snake case and answers what its namesake answers. Every export is declared in its module's `exports!` block under its TypeScript name, and `crates/agent-plugins/tests/differential/registry.rs` holds every declared export to every recorded call, called by name as the wrapper calls it. A call with no Rust spelling (an `undefined` where the parameter is no `JsValue` or `Option`) is counted and left out; nothing else is. |
| RS3 | No I/O, no ambient state: files, commands, requests, the environment and the clock are ports (`crates/agent-plugins/src/ports/`), traits whose methods return a `BoxFuture`. The crate never spawns, sleeps or blocks (clippy.toml's disallowed list). |
| RS4 | No panic: no `unwrap`, `expect`, `panic!`, indexing that can fail, or slicing inside a character. A plugin's mistake is a `PluginIssue`, as in the package. |
| RS5 | The lints in `Cargo.toml` are hard. An exception is an `#[expect(lint, reason = "...")]` on the smallest item. At most 300 lines of code per file, 100 per function; `x.rs` beside `x/`, never `mod.rs`. |
| RS6 | JavaScript's semantics where they reach a result live in `crates/agent-plugins/src/js.rs`: UTF-16 length and order (`code_unit_length`, `compare_code_units`), `trim`'s blanks, a number's text. A regular expression keeps JavaScript's classes: `\d`, `\w` and `\b` are ASCII there (spell them `[0-9]`, `[A-Za-z0-9_]` or use `(?-u:...)`), `\s` is `js::is_whitespace`'s set. |
| RS7 | Shapes are the package's JSON: an interface is a struct with `#[serde(rename_all = "camelCase")]`, an optional member an `Option` that is skipped when `None`, a union of strings an enum renamed to its strings, a tagged union an enum with `#[serde(tag = "...")]`. A value the package types `unknown`, or reads from JSON or YAML, is a `js::JsValue` (`undefined`, `NaN`, the infinities and `-0` included; `JsValue::Undefined` is a missing argument); a `number` field that may hold one of those four is serialized with `js::value::serialize_number`. Member order is the package's: a `JsObject` keeps order, maps are `IndexMap`. The library reads and writes JSON itself: `clippy.toml` disallows `serde_json::Value` and `serde_json::from_str`, which the differential tests use to read the vectors, and `js::json_value` alone builds a `Value` for flowschema. |
| RS9 | A port mirrors its source so a TypeScript diff applies to it at a glance: the same functions in the same order, the same names, tables copied verbatim in the same order, the spec citations of the doc comments kept. Its first line is its port header, stamped by `bun scripts/rust/ports.ts --stamp <file>` when the port matches its source; `ports.ts --check` keeps it honest. |
| RS8 | A dependency earns its place, with its reason in a comment beside it in `Cargo.toml`: `serde`, `indexmap`, `regex`, `regress` (ECMAScript's own regular expressions, lookaround and backreferences, on UTF-16, for a pattern from input: a hook's matcher, a relevance signal), `url` (the WHATWG URL parser, so no URL parser of our own is kept current), `unicode-normalization` (`String.prototype.normalize`'s four forms, so no Unicode table of our own is kept current) and `flowschema` (a value checked against a JSON Schema, an MCP tool's input schema against the official draft 2020-12 meta-schema, so no keyword table of our own is kept current; at a commit of its main, as llx takes it) so far; `serde_json` because flowschema reads its `Value`, built by `js::json_value` alone, and in the tests. The npm package's `yaml` is ported by hand instead (`crates/agent-plugins/src/yaml.rs`): no YAML crate words and places its errors as the package does, keeps the comments an edit must keep, or writes what its `stringify` writes. |

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
- **YAML nested past 128 collections is refused** with the package's own error for its stack
  running out (`Maximum call stack size exceeded.`), read or written, where Bun's stack lets the
  package go past 5,000 levels. A level costs the core about 1.4 KiB of WebAssembly's 1 MiB stack
  and about 8 KiB in a debug build (`crates/agent-plugins/src/yaml.rs`, `MAX_DEPTH`). No
  frontmatter comes near it.
- **A workflow script nested past 128 levels** (`${...}` inside templates, arrays and objects
  inside the `meta` literal) is the package's own problem for its stack running out (`code nested
  too deep to scan (Maximum call stack size exceeded.)`, `… to read (…)`), where Bun's stack lets
  the package go thousands of levels deeper (`format/js_tokens.rs`, `format/js_literal.rs`,
  `MAX_NESTING`). No script comes near it.
- **Brace groups nested past 128 in a `paths` pattern** (`{a}{b}…` or `{a,{b,…}}`) are the
  package's own error for its stack running out (`Maximum call stack size exceeded.`), where Bun's
  stack lets the expansion go deeper (`skills/path_gate.rs`, `MAX_NESTING`). No pattern comes near it.
- **YAML beyond the core schema**: the frontmatter reads the YAML 1.2 core schema, and so does the
  core, always. The package also knows `!!binary`, `!!set`, `!!omap`, `!!pairs` and
  `!!timestamp` (a `Uint8Array`, a `Set`, a `Map`, a `Date`), and reads a `%YAML 1.1` document
  (`%YAML 1.1` then `--- {x: yes}`) with YAML 1.1's schema (`yes` is `true`); the core reads such
  a value as its untagged self (`"aGk="`, `{"a": null}`, `"2001-12-14"`) and `yes` as text.
- **Unicode is the crate's version, 18, where Bun's ICU (78) is Unicode 17.** `normalize` (the
  `unicode-normalization` crate) and `toUpperCase` and `toLowerCase` (Rust's own) know the
  characters Unicode 18 added, which Bun leaves as they are: 68 characters with a compatibility
  decomposition (`NFKC`, `NFKD`; U+0558, U+058B, U+058C, U+209D–U+209F, U+107BB–U+107BF,
  U+1B123–U+1B126 among them) and about 20 case pairs (U+A7DD, U+A7E2, U+AB4B, U+AB4C,
  U+1DF40–U+1DF75 among them, and the new capital of U+027C). `NFC` and `NFD` agree on every code
  point. An archive's names are folded with them (`marketplace/archive-scan`), where a new pair
  can only fold two names together and refuse more. The oracle's texts leave these characters out.
- **A YAML alias inside its own anchor** (`x: &a [*a]`) reads as `[[]]`: the package gives an
  array that holds itself, which no value crossing the boundary can be.
- **A regular expression is JavaScriptCore's, not the spec's, where the two differ**
  (`js::regexp`): with `u` or `v` an escaped character past ASCII (`\é`) is that character, as
  Bun reads it (the spec and V8 refuse it). Five corners stay the engine's: a `v` pattern holding
  what JavaScriptCore refuses (`\q`, `\00`, a lone `]`, `{` or `}`, `\c1`, `\1` with no group)
  compiles; without `u`, `i` relates the letters Unicode 16 added to their case partners (U+019B,
  U+0264, U+1C89, U+1C8A, U+A7CB–U+A7DC), whose pattern case table in JavaScriptCore is older than
  its `toUpperCase`; a pattern without `u` whose backreference `i` applies to and which also turns
  `i` on or off for a group folds by Unicode's case folding (`ſ` meets `s`), not by
  `Canonicalize`; a lone surrogate in a capture reads as U+FFFD (above); and the reason of a
  `SyntaxError` is read from the engine's error and the pattern's text, so a pattern the oracle
  never met may name another reason than JavaScriptCore would. No module compiles with `v`, and
  none shows the reason. Neither engine bounds backtracking (`/(a+)+$/` on a text of 26 `a`s took
  JavaScriptCore 0.6 s): the core adds no limit the spec does not state.

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
