//! The schemas of `scripts/rust/js-oracle/zod-schemas.ts`, built in Rust under the same names in
//! the same order, each line beside its TypeScript: the scalars and objects here, the unions,
//! refinements and transforms in `zod_schemas_refined.rs`.

use agent_plugins::js::JsValue;
use agent_plugins::zod::{self, Params, RawIssue, Schema, error_fn, fields};

use super::zod_schemas_refined::refined;

fn wrong_value(expected: &'static str) -> Params {
    error_fn(move |found: &RawIssue| {
        if found.input_value().is_undefined() {
            "is required".to_owned()
        } else {
            format!("expected {expected}")
        }
    })
}

fn source_xor_content(entry: &JsValue) -> bool {
    let has = |key: &str| entry.get(key).is_some_and(|value| !value.is_undefined());
    has("source") != has("content")
}

pub(super) fn schemas() -> Vec<(&'static str, Schema)> {
    let mut all = scalars();
    all.extend(objects());
    all.extend(refined());
    all
}

fn scalars() -> Vec<(&'static str, Schema)> {
    vec![
        ("string", zod::string()),
        ("number", zod::number()),
        ("boolean", zod::boolean()),
        ("unknown", zod::unknown()),
        ("int", zod::int()),
        ("url", zod::url()),
        ("json", zod::json()),
        ("literal", zod::literal("command")),
        ("literal.number", zod::literal(0.0)),
        (
            "literal.error",
            zod::literal("text").error("expected \"text\""),
        ),
        ("enum", zod::r#enum(["stdio", "socket"])),
        ("enum.one", zod::r#enum(["only"])),
        ("nullable", zod::string().nullable()),
        ("optional", zod::string().optional()),
        ("exactOptional", zod::string().exact_optional()),
        ("string.min", zod::string().min(1.0)),
        ("string.max", zod::string().max(3.0)),
        ("string.min.max", zod::string().min(2.0).max(4.0)),
        (
            "string.min.error",
            zod::string().min_with(1.0, "expected a string"),
        ),
        ("string.regex", zod::string().regex(r"^\.")),
        (
            "string.regex.key",
            zod::string().regex("^[A-Za-z_][A-Za-z0-9_]*$"),
        ),
        ("number.positive", zod::number().positive()),
        ("number.nonnegative", zod::number().nonnegative()),
        ("number.max", zod::number().max(10.0)),
        ("number.int", zod::number().int()),
        ("number.int.positive", zod::number().int().positive()),
        ("number.int.min", zod::number().int().min(0.0)),
        ("int.min.max", zod::int().min(1.0).max(65535.0)),
        ("int.nonnegative", zod::int().nonnegative()),
        (
            "number.errors",
            zod::number()
                .error("must be a positive number")
                .positive_with("must be a positive number")
                .finite(),
        ),
        ("array", zod::array(zod::string())),
        ("array.min", zod::array(zod::string()).min(1.0)),
        ("array.max", zod::array(zod::string().max(3.0)).max(2.0)),
        ("array.method", zod::string().array()),
        ("record", zod::record(zod::string(), zod::string())),
        ("record.unknown", zod::record(zod::string(), zod::unknown())),
    ]
}

#[expect(
    clippy::too_many_lines,
    reason = "one line per schema of the oracle, in its order"
)]
fn objects() -> Vec<(&'static str, Schema)> {
    vec![
        (
            "record.key-regex",
            zod::record(zod::string().regex(r"^\."), zod::string()).refine(
                |map| map.as_object().is_some_and(|members| !members.is_empty()),
                "needs one mapping",
            ),
        ),
        (
            "record.enum-values",
            zod::record(zod::string(), zod::r#enum(["a", "b"])),
        ),
        (
            "record.enum-keys",
            zod::record(zod::r#enum(["a", "b"]), zod::number()),
        ),
        (
            "object",
            zod::object(fields! { "name" => zod::string(), "count" => zod::number().optional() }),
        ),
        (
            "object.loose",
            zod::loose_object(
                fields! { "name" => zod::string(), "flag" => zod::boolean().optional() },
            ),
        ),
        (
            "object.strict",
            zod::strict_object(fields! { "name" => zod::string() }),
        ),
        (
            "object.unknown",
            zod::object(fields! { "value" => zod::unknown() }),
        ),
        (
            "object.exact",
            zod::object(fields! {
                "name" => zod::string().exact_optional(),
                "value" => zod::unknown().exact_optional(),
            }),
        ),
        (
            "object.nullable",
            zod::object(
                fields! { "name" => zod::string().nullable(), "b" => zod::string().optional() },
            ),
        ),
        (
            "object.nested",
            zod::object(fields! {
                "author" => zod::object(
                    fields! { "name" => zod::string(), "email" => zod::string().optional() },
                ),
                "tags" => zod::array(zod::string()),
            }),
        ),
        (
            "object.partial",
            zod::object(fields! { "name" => zod::string(), "version" => zod::string() }).partial(),
        ),
        (
            "object.required",
            zod::object(fields! { "name" => zod::string(), "version" => zod::string() })
                .partial()
                .required(&["name"]),
        ),
        (
            "object.extend",
            zod::object(fields! { "name" => zod::string() })
                .extend(fields! { "b" => zod::number() }),
        ),
        (
            "object.indexes",
            zod::object(fields! {
                "2" => zod::string(), "b" => zod::string().optional(), "1" => zod::string(),
            }),
        ),
        (
            "object.refine-path",
            zod::object(fields! {
                "source" => zod::string().optional(), "content" => zod::string().optional(),
            })
            .refine(
                source_xor_content,
                Params::from("exactly one of source or content").path(["source"]),
            ),
        ),
        (
            "object.refine",
            zod::object(fields! {
                "source" => zod::string().optional(), "content" => zod::string().optional(),
            })
            .refine(source_xor_content, "exactly one of source or content"),
        ),
        (
            "object.error-fn",
            zod::object(fields! {
                "name" => zod::string().error(wrong_value("a string")),
                "count" => zod::number().error(wrong_value("a number")).optional(),
                "tags" => zod::array(zod::string()).error(wrong_value("an array")).optional(),
                "mode" => zod::r#enum(["a", "b"])
                    .error(error_fn(|found| {
                        if found.input_value().is_undefined() {
                            "is required".to_owned()
                        } else {
                            "expected one of a, b".to_owned()
                        }
                    }))
                    .optional(),
            })
            .error(error_fn(|found| {
                if found.input_value().is_undefined() {
                    "is required".to_owned()
                } else {
                    "expected an object".to_owned()
                }
            })),
        ),
    ]
}
