# Changelog

## Unreleased

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
