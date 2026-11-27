//! The unions, refinements, checks, transforms, defaults and catches of
//! `scripts/rust/js-oracle/zod-schemas.ts`, in its order after the objects (`zod_schemas.rs`).

use std::collections::HashSet;

use agent_plugins::js::{JsValue, code_unit_length, is_whitespace, url};
use agent_plugins::zod::{self, Schema, fields, issue};

fn is_https(value: &JsValue) -> bool {
    value
        .as_str()
        .and_then(url::parse)
        .is_some_and(|found| found.protocol == "https:")
}

fn closed_object(shape: zod::Shape) -> Schema {
    let known: Vec<String> = shape.keys().cloned().collect();
    zod::loose_object(shape).check(move |context| {
        let keys: Vec<JsValue> = context
            .value()
            .as_object()
            .map(|members| {
                members
                    .keys()
                    .filter(|key| !known.contains(key))
                    .map(|key| JsValue::from(key.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        if keys.is_empty() {
            return;
        }
        let input = context.value().clone();
        context.push_issue(
            issue("unrecognized_keys")
                .set("keys", keys)
                .input(input)
                .continues(true),
        );
    })
}

const MODES: [&str; 2] = ["default", "acceptEdits"];
const ON_SKILL: &str = "on-skill-invoke:";

fn text(value: &JsValue) -> &str {
    value.as_str().unwrap_or_default()
}

fn unique_names(entries: &JsValue) -> bool {
    let list = entries.as_array().map_or(&[][..], Vec::as_slice);
    let names: HashSet<String> = list
        .iter()
        .map(|entry| {
            entry
                .get("name")
                .map(|name| format!("{name:?}"))
                .unwrap_or_default()
        })
        .collect();
    names.len() == list.len()
}

fn monitor_entry() -> zod::Shape {
    fields! { "name" => zod::string().min(1.0), "when" => zod::string().exact_optional() }
}

#[expect(
    clippy::too_many_lines,
    reason = "one line per schema of the oracle, in its order"
)]
pub(super) fn refined() -> Vec<(&'static str, Schema)> {
    vec![
        (
            "union",
            zod::union([zod::string(), zod::array(zod::string())]),
        ),
        (
            "union.objects",
            zod::union([
                zod::object(fields! { "kind" => zod::literal("a"), "a" => zod::string() }),
                zod::object(fields! { "kind" => zod::literal("b"), "b" => zod::number() }),
            ]),
        ),
        (
            "union.refined-list",
            zod::union([
                zod::string(),
                zod::array(closed_object(monitor_entry()))
                    .refine(unique_names, "names must be unique"),
            ]),
        ),
        (
            "union.nested",
            zod::union([
                zod::string(),
                zod::record(zod::string(), zod::string()),
                zod::array(zod::union([
                    zod::string(),
                    zod::record(zod::string(), zod::string()),
                ])),
            ]),
        ),
        (
            "union.scalars",
            zod::union([
                zod::string(),
                zod::number(),
                zod::boolean(),
                zod::array(zod::string()),
            ])
            .optional(),
        ),
        (
            "discriminated",
            zod::discriminated_union(
                "type",
                [
                    zod::object(
                        fields! { "type" => zod::literal("setMode"), "mode" => zod::string() },
                    ),
                    zod::object(fields! {
                        "type" => zod::literal("addDirectories"),
                        "directories" => zod::array(zod::string()),
                    }),
                ],
            ),
        ),
        (
            "refine.when",
            zod::string()
                .refine(
                    |value| text(value) == "always" || text(value).starts_with(ON_SKILL),
                    format!("Invalid string: must start with \"{ON_SKILL}\""),
                )
                .refine(
                    |value| {
                        text(value) == "always"
                            || text(value)
                                .strip_prefix(ON_SKILL)
                                .map_or(0, code_unit_length)
                                > 0
                    },
                    "on-skill-invoke: must specify a skill name",
                ),
        ),
        (
            "min.refine",
            zod::string().min(1.0).refine(
                |value| text(value).starts_with('/') || !text(value).chars().any(is_whitespace),
                "Command should not contain spaces.",
            ),
        ),
        (
            "refine.no-params",
            zod::string().refine(|value| text(value).find('@') == text(value).rfind('@'), ()),
        ),
        (
            "superRefine",
            zod::strict_object(fields! {
                "type" => zod::r#enum(["string", "number"]),
                "default" => zod::string().optional(),
            })
            .super_refine(|option, context| {
                let typed_number = option.get("type").and_then(JsValue::as_str) == Some("number");
                let has_default = option
                    .get("default")
                    .is_some_and(|value| !value.is_undefined());
                if typed_number && has_default {
                    context.add_issue(
                        issue("custom")
                            .path(["default"])
                            .message("default must be a number"),
                    );
                }
            }),
        ),
        ("check.unrecognized", closed_object(monitor_entry())),
        (
            "check.custom",
            zod::string().check(|context| {
                let value = text(context.value()).to_owned();
                if agent_plugins::js::trim(&value) == value {
                    return;
                }
                context.push_issue(
                    issue("custom")
                        .input(JsValue::from(value))
                        .message("An option cannot start or end with a space")
                        .continues(true),
                );
            }),
        ),
        (
            "transform",
            zod::string().transform(|raw, context| {
                if MODES.contains(&text(raw)) {
                    return raw.clone();
                }
                context.add_issue(
                    issue("custom").message(format!("\"{}\" is not a permission mode", text(raw))),
                );
                zod::NEVER
            }),
        ),
        (
            "transform.length",
            zod::string().transform(|value, _context| {
                #[expect(clippy::cast_precision_loss, reason = "a length is below 2^53")]
                JsValue::Number(code_unit_length(text(value)) as f64)
            }),
        ),
        ("default", zod::string().default("x")),
        (
            "default.object",
            zod::object(fields! {
                "mode" => zod::r#enum(["a", "b"]).default("a"),
                "count" => zod::number().default(1.0),
            }),
        ),
        ("catch", zod::number().catch(0.0)),
        (
            "catch.object",
            zod::object(fields! {
                "count" => zod::number().catch(-1.0),
                "name" => zod::string().optional().catch("none"),
            }),
        ),
        (
            "custom",
            zod::custom(|value| !value.is_undefined(), "Required"),
        ),
        (
            "custom.object",
            zod::object(fields! {
                "value" => zod::custom(|value| !value.is_undefined(), "Required").exact_optional(),
                "entry" => zod::custom(
                    |value| {
                        zod::loose_object(fields! { "type" => zod::string() })
                            .safe_parse(value)
                            .is_ok()
                    },
                    "Expected a permission update entry",
                ),
            }),
        ),
        (
            "refine.optional",
            zod::string()
                .refine(is_https, "must use https://")
                .optional(),
        ),
        (
            "url.object",
            zod::object(fields! {
                "url" => zod::url(),
                "headers" => zod::record(zod::string(), zod::string()).optional(),
            }),
        ),
        ("instanceof", zod::instanceof("Array", JsValue::is_array)),
    ]
}
