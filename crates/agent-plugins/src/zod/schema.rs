//! A schema: what it is (`Kind`, zod's `def.type` with its parts), the checks it runs after it
//! parses (`def.checks`), and its error map (`def.error`). A schema is shared, never changed: a
//! method returns a new one, as zod's `clone` does, so a schema built once is used everywhere.

use std::sync::{Arc, OnceLock};

use regex::Regex;

use super::context::{CatchContext, Context};
use super::params::{ErrorMap, Shape};
use crate::js::JsValue;

/// A zod schema (the module comment of [`crate::zod`]).
#[derive(Clone)]
pub struct Schema(pub(crate) Arc<Node>);

impl std::fmt::Debug for Schema {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Schema")
    }
}

#[derive(Clone)]
pub(crate) struct Node {
    pub(crate) kind: Kind,
    pub(crate) checks: Vec<Check>,
    pub(crate) error: Option<ErrorMap>,
}

pub(crate) type Test = Arc<dyn Fn(&JsValue) -> bool + Send + Sync>;
pub(crate) type CheckFn = Arc<dyn Fn(&mut Context<'_>) + Send + Sync>;
pub(crate) type RefineFn = Arc<dyn Fn(&JsValue, &mut Context<'_>) + Send + Sync>;
pub(crate) type TransformFn = Arc<dyn Fn(&JsValue, &mut Context<'_>) -> JsValue + Send + Sync>;
pub(crate) type CatchFn = Arc<dyn Fn(&CatchContext) -> JsValue + Send + Sync>;

/// A `z.lazy` schema: its getter, called once, on the first parse that reaches it.
pub(crate) struct Lazy {
    pub(crate) getter: fn() -> Schema,
    pub(crate) inner: OnceLock<Schema>,
}

impl Lazy {
    pub(crate) fn inner(&self) -> &Schema {
        self.inner.get_or_init(self.getter)
    }
}

/// What a schema is, zod's `def.type`.
#[derive(Clone)]
pub(crate) enum Kind {
    String,
    Number,
    Boolean,
    Unknown,
    Never,
    Null,
    /// `z.literal(values)` and `z.enum(values)`, which differ only in `values` (zod's `values`).
    Literal(Vec<JsValue>),
    Enum(Vec<JsValue>),
    Array(Schema),
    /// An object: `catchall` is `None` for `z.object` (unknown keys dropped), `unknown()` for
    /// `z.looseObject` (kept), `never()` for `z.strictObject` (refused).
    Object {
        shape: Shape,
        catchall: Option<Schema>,
    },
    Record {
        key: Schema,
        value: Schema,
    },
    Union(Vec<Schema>),
    DiscriminatedUnion {
        discriminator: String,
        options: Vec<Schema>,
    },
    Optional(Schema),
    ExactOptional(Schema),
    Nullable(Schema),
    NonOptional(Schema),
    Default(Schema, JsValue),
    Catch(Schema, CatchFn),
    Pipe(Schema, Schema),
    Transform(TransformFn),
    /// `z.custom(test)` and `z.instanceof`: it accepts anything, its first check decides.
    Custom,
    Lazy(Arc<Lazy>),
}

/// A check a schema runs after it parses: zod's `$ZodCheck`, with its error map, its `abort`
/// and, for a refinement, its `path`.
#[derive(Clone)]
pub(crate) struct Check {
    pub(crate) kind: CheckKind,
    pub(crate) error: Option<ErrorMap>,
    pub(crate) abort: bool,
    pub(crate) path: Vec<JsValue>,
}

#[derive(Clone)]
pub(crate) enum CheckKind {
    MinLength(f64),
    MaxLength(f64),
    GreaterThan {
        value: f64,
        inclusive: bool,
    },
    LessThan {
        value: f64,
        inclusive: bool,
    },
    /// `z.int()`, `.int()`: zod's `safeint` number format.
    SafeInt,
    /// `.regex(pattern)`: the pattern as JavaScript writes it (`/^\./`) and as Rust runs it.
    Regex {
        literal: String,
        regex: Option<Arc<Regex>>,
    },
    /// `z.url()`'s own check: `new URL(text.trim())`.
    Url,
    /// `.refine(test)` and `z.custom(test)`.
    Refine(Test),
    /// `z.instanceof(Class)`: an `invalid_type` naming the class.
    Instance {
        name: &'static str,
        test: Test,
    },
    /// `.check((context) => …)`.
    Function(CheckFn),
    /// `.superRefine((value, context) => …)`.
    SuperRefine(RefineFn),
}

impl Check {
    pub(crate) const fn new(kind: CheckKind, error: Option<ErrorMap>) -> Self {
        Self {
            kind,
            error,
            abort: false,
            path: Vec::new(),
        }
    }

    /// Whether it runs on a value with a `length` even after an issue (zod's `when` of the
    /// length checks), rather than only while nothing aborted.
    pub(crate) const fn runs_on_length(&self) -> bool {
        matches!(self.kind, CheckKind::MinLength(_) | CheckKind::MaxLength(_))
    }
}

impl Schema {
    pub(crate) fn of(kind: Kind) -> Self {
        Self(Arc::new(Node {
            kind,
            checks: Vec::new(),
            error: None,
        }))
    }

    pub(crate) fn kind(&self) -> &Kind {
        &self.0.kind
    }

    /// The schema with one more check, as `.check(...)` clones it.
    pub(crate) fn with_check(&self, check: Check) -> Self {
        let mut node = (*self.0).clone();
        node.checks.push(check);
        Self(Arc::new(node))
    }

    /// zod's `optin === "optional"`: a missing member is no issue of the object's.
    pub(crate) fn optional_in(&self) -> bool {
        match self.kind() {
            Kind::Optional(_)
            | Kind::ExactOptional(_)
            | Kind::Transform(_)
            | Kind::Default(..)
            | Kind::Catch(..) => true,
            Kind::Nullable(inner) => inner.optional_in(),
            Kind::Union(options) | Kind::DiscriminatedUnion { options, .. } => {
                options.iter().any(Self::optional_in)
            }
            Kind::Pipe(first, _) => first.optional_in(),
            Kind::Lazy(lazy) => lazy.inner().optional_in(),
            Kind::String
            | Kind::Number
            | Kind::Boolean
            | Kind::Unknown
            | Kind::Never
            | Kind::Null
            | Kind::Literal(_)
            | Kind::Enum(_)
            | Kind::Array(_)
            | Kind::Object { .. }
            | Kind::Record { .. }
            | Kind::NonOptional(_)
            | Kind::Custom => false,
        }
    }

    /// zod's `optout === "optional"`: the output may lack the member.
    pub(crate) fn optional_out(&self) -> bool {
        match self.kind() {
            Kind::Optional(_) | Kind::ExactOptional(_) => true,
            Kind::Nullable(inner) | Kind::Catch(inner, _) => inner.optional_out(),
            Kind::Union(options) | Kind::DiscriminatedUnion { options, .. } => {
                options.iter().any(Self::optional_out)
            }
            Kind::Pipe(_, last) => last.optional_out(),
            Kind::Lazy(lazy) => lazy.inner().optional_out(),
            Kind::String
            | Kind::Number
            | Kind::Boolean
            | Kind::Unknown
            | Kind::Never
            | Kind::Null
            | Kind::Literal(_)
            | Kind::Enum(_)
            | Kind::Array(_)
            | Kind::Object { .. }
            | Kind::Record { .. }
            | Kind::NonOptional(_)
            | Kind::Default(..)
            | Kind::Transform(_)
            | Kind::Custom => false,
        }
    }

    /// zod's `values`: every value the schema accepts, when it accepts only a few (a literal, an
    /// enum, and what wraps them), which a discriminated union and a record's key read.
    pub(crate) fn values(&self) -> Option<Vec<JsValue>> {
        match self.kind() {
            Kind::Literal(values) | Kind::Enum(values) => Some(values.clone()),
            Kind::Optional(inner) => inner
                .values()
                .map(|values| with(values, JsValue::Undefined)),
            Kind::Nullable(inner) => inner.values().map(|values| with(values, JsValue::Null)),
            Kind::ExactOptional(inner) | Kind::Default(inner, _) | Kind::Catch(inner, _) => {
                inner.values()
            }
            Kind::NonOptional(inner) => inner.values().map(|values| {
                values
                    .into_iter()
                    .filter(|value| !value.is_undefined())
                    .collect()
            }),
            Kind::Pipe(first, _) => first.values(),
            Kind::Union(options) => options
                .iter()
                .map(Self::values)
                .collect::<Option<Vec<_>>>()
                .map(|lists| {
                    lists
                        .into_iter()
                        .fold(Vec::new(), |all, list| list.into_iter().fold(all, with))
                }),
            Kind::String
            | Kind::Number
            | Kind::Boolean
            | Kind::Unknown
            | Kind::Never
            | Kind::Null
            | Kind::Array(_)
            | Kind::Object { .. }
            | Kind::Record { .. }
            | Kind::DiscriminatedUnion { .. }
            | Kind::Transform(_)
            | Kind::Custom
            | Kind::Lazy(_) => None,
        }
    }
}

/// A `Set`'s add: `value` at the end unless it is there (`SameValueZero`).
fn with(mut values: Vec<JsValue>, value: JsValue) -> Vec<JsValue> {
    if !values.iter().any(|known| same_value_zero(known, &value)) {
        values.push(value);
    }
    values
}

/// JavaScript's `SameValueZero`, how a `Set` and a `Map` compare keys: `NaN` is itself, `-0` is
/// `0`, an array or an object only itself (so never one read from input).
pub(crate) fn same_value_zero(first: &JsValue, second: &JsValue) -> bool {
    match (first, second) {
        (JsValue::Number(a), JsValue::Number(b)) => a == b || (a.is_nan() && b.is_nan()),
        (JsValue::Undefined, JsValue::Undefined) | (JsValue::Null, JsValue::Null) => true,
        (JsValue::Bool(a), JsValue::Bool(b)) => a == b,
        (JsValue::String(a), JsValue::String(b)) => a == b,
        _ => false,
    }
}
