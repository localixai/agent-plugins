# Changelog

## Unreleased

- `registry::Port` implements every port trait (`registry/remote/`). The methods the TypeScript
  ports answer at once (`Clock::now`, `Environment::get` and `snapshot`, `Locations::home_dir`,
  `project_dir`, `plugin_root` and `join_path`, `FolderScan::managed_directory`, and
  `ScanFolder`'s `dir`, `files` and `host_path`) return a `BoxFuture` now, as a host across the
  boundary answers them as needs; `Clock::set_timer` answers its cancel as a future, and the
  cancel is one. `registry::Rest` is a rest parameter, `registry::Function` a function of the
  core's the host calls, `Port::member` reads a member of the host's object.
- The crate: the ports, the JavaScript semantics the package relies on, and the first `format`
  modules, held to the vectors recorded from the npm package.
- `js::json_parse` is JavaScriptCore's `JSON.parse`, its error messages included, and `js::number`
  is `Number(text)`; both, with `js::number_to_string` and `js::trim`, are held to the built-ins
  as `scripts/rust/js-oracle.ts` records them in Bun. A text nested past 1000 levels is refused.
- `js::JsValue` is a JavaScript value, what the package types `unknown`: `undefined`, `NaN`, the
  infinities and `-0` included. `js::json_parse` returns one (`JSON.parse("-0")` is `-0`, `1e400`
  is `Infinity`), the registry takes and answers them, tagged on the wire as
  `{"$undefined": true}` and `{"$number": "NaN" | "Infinity" | "-Infinity" | "-0"}`, and writes
  its JSON itself, each number as JavaScript writes it. The library no longer depends on
  `serde_json`; `registry::Unknown` is gone (`JsValue::Undefined`).
