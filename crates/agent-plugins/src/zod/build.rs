//! The builders, named as zod names them (`z.string()` is `zod::string()`), each the schema its
//! namesake makes. A builder's params (`z.string({ error })`) are its `.error(params)`.

use std::sync::{Arc, OnceLock};

use regex::Regex;

use super::params::{Params, Shape};
use super::schema::{Check, CheckKind, Kind, Lazy, Schema};
use crate::js::{JsValue, in_key_order};

/// `z.string()`.
#[must_use]
pub fn string() -> Schema {
    Schema::of(Kind::String)
}

/// `z.number()`: a finite number.
#[must_use]
pub fn number() -> Schema {
    Schema::of(Kind::Number)
}

/// `z.int()`: a number that is a safe integer.
#[must_use]
pub fn int() -> Schema {
    number().with_check(Check::new(CheckKind::SafeInt, None))
}

/// `z.boolean()`.
#[must_use]
pub fn boolean() -> Schema {
    Schema::of(Kind::Boolean)
}

/// `z.unknown()`: any value, `undefined` too (but a missing member is still an issue of an
/// object's unless it is `.optional()`).
#[must_use]
pub fn unknown() -> Schema {
    Schema::of(Kind::Unknown)
}

/// `z.never()`: no value.
#[must_use]
pub fn never() -> Schema {
    Schema::of(Kind::Never)
}

/// `z.null()`.
#[must_use]
pub fn null() -> Schema {
    Schema::of(Kind::Null)
}

/// `z.url()`: a string `new URL` reads once trimmed; the output is the trimmed text.
#[must_use]
pub fn url() -> Schema {
    string().with_check(Check::new(CheckKind::Url, None))
}

/// `z.literal(value)`.
#[must_use]
pub fn literal(value: impl Into<JsValue>) -> Schema {
    Schema::of(Kind::Literal(vec![value.into()]))
}

/// `z.enum([...])`: one of the values, in the order given.
#[must_use]
pub fn r#enum<V: Into<JsValue>>(values: impl IntoIterator<Item = V>) -> Schema {
    Schema::of(Kind::Enum(values.into_iter().map(Into::into).collect()))
}

/// `z.array(element)`.
#[must_use]
pub fn array(element: Schema) -> Schema {
    Schema::of(Kind::Array(element))
}

fn object_of(shape: Shape, catchall: Option<Schema>) -> Schema {
    Schema::of(Kind::Object {
        shape: in_key_order(shape),
        catchall,
    })
}

/// `z.object(shape)`: the shape's members; a key it does not name is dropped.
#[must_use]
pub fn object(shape: Shape) -> Schema {
    object_of(shape, None)
}

/// `z.looseObject(shape)`: the shape's members, and every other key kept as it is.
#[must_use]
pub fn loose_object(shape: Shape) -> Schema {
    object_of(shape, Some(unknown()))
}

/// `z.strictObject(shape)`: the shape's members; any other key is an `unrecognized_keys` issue.
#[must_use]
pub fn strict_object(shape: Shape) -> Schema {
    object_of(shape, Some(never()))
}

/// `z.record(key, value)`: an object whose every key passes `key` and every value `value`; a key
/// schema of a few values (an enum) asks for each of them.
#[must_use]
pub fn record(key: Schema, value: Schema) -> Schema {
    Schema::of(Kind::Record { key, value })
}

/// `z.union([...])`.
#[must_use]
pub fn union(options: impl IntoIterator<Item = Schema>) -> Schema {
    Schema::of(Kind::Union(options.into_iter().collect()))
}

/// `z.discriminatedUnion(discriminator, [...])`: objects told apart by the literal values of one
/// member.
#[must_use]
pub fn discriminated_union(
    discriminator: &str,
    options: impl IntoIterator<Item = Schema>,
) -> Schema {
    Schema::of(Kind::DiscriminatedUnion {
        discriminator: discriminator.to_owned(),
        options: options.into_iter().collect(),
    })
}

/// `z.custom(test, params)`: any value `test` accepts; a failure aborts the checks after it.
#[must_use]
pub fn custom(
    test: impl Fn(&JsValue) -> bool + Send + Sync + 'static,
    params: impl Into<Params>,
) -> Schema {
    let params = params.into();
    let mut check = Check::new(CheckKind::Refine(Arc::new(test)), params.error_map());
    check.abort = params.aborts(true);
    check.path = params.refine_path();
    Schema::of(Kind::Custom).with_check(check)
}

/// `z.instanceof(Class)`: a value of the class `name`, which `test` recognizes (a JavaScript
/// value read from JSON is an `Array` or an `Object`).
#[must_use]
pub fn instanceof(
    name: &'static str,
    test: impl Fn(&JsValue) -> bool + Send + Sync + 'static,
) -> Schema {
    let mut check = Check::new(
        CheckKind::Instance {
            name,
            test: Arc::new(test),
        },
        None,
    );
    check.abort = true;
    Schema::of(Kind::Custom).with_check(check)
}

/// `z.lazy(getter)`: the schema `getter` builds, built on the first parse that reaches it, so a
/// schema can hold itself.
#[must_use]
pub fn lazy(getter: fn() -> Schema) -> Schema {
    Schema::of(Kind::Lazy(Arc::new(Lazy {
        getter,
        inner: OnceLock::new(),
    })))
}

fn json_value() -> Schema {
    union([
        string(),
        number(),
        boolean(),
        null(),
        array(json()),
        record(string(), json()),
    ])
}

/// `z.json()`: a value JSON can write, at any depth (a finite number, no `undefined`).
#[must_use]
pub fn json() -> Schema {
    lazy(json_value)
}

/// A regular expression as `.regex` takes it: the JavaScript literal's pattern, which Rust's
/// `regex` must read as JavaScript does (RS6), and its flags; `None` when it does not compile,
/// which a unit test of the schema catches.
pub(crate) fn pattern(source: &str, flags: &str) -> (String, Option<Arc<Regex>>) {
    let inline = if flags.contains('i') { "(?i)" } else { "" };
    let compiled = Regex::new(&format!("{inline}{source}")).ok().map(Arc::new);
    (format!("/{source}/{flags}"), compiled)
}
