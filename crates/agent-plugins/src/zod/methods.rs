//! A schema's methods, named as zod names them (`.optional()`, `.min(n)`, `.refine(test, params)`),
//! each returning a new schema. A method's params (`.min(1, { error })`) are its `_with` twin's
//! second argument, a refinement's always its second.

use std::sync::Arc;

use super::build::{array, pattern};
use super::context::{CatchContext, Context};
use super::issue::ZodError;
use super::params::{Params, Shape};
use super::run::{Payload, run};
use super::schema::{Check, CheckKind, Kind, Node, Schema};
use crate::js::{JsValue, in_key_order};

impl Schema {
    fn wrap(&self, kind: impl FnOnce(Self) -> Kind) -> Self {
        Self::of(kind(self.clone()))
    }

    fn checked(&self, kind: CheckKind, params: impl Into<Params>) -> Self {
        let params = params.into();
        let mut check = Check::new(kind, params.error_map());
        check.abort = params.aborts(false);
        check.path = params.refine_path();
        self.with_check(check)
    }

    /// The builder's params, `z.string({ error })`: the error map of the issues this schema
    /// raises itself (not those of its checks or its members).
    #[must_use]
    pub fn error(&self, params: impl Into<Params>) -> Self {
        let mut node: Node = (*self.0).clone();
        node.error = params.into().error_map();
        Self(Arc::new(node))
    }

    /// `.optional()`: `undefined` too, and a member that may be missing.
    #[must_use]
    pub fn optional(&self) -> Self {
        self.wrap(Kind::Optional)
    }

    /// `.exactOptional()`: a member that may be missing, but not `undefined` when present.
    #[must_use]
    pub fn exact_optional(&self) -> Self {
        self.wrap(Kind::ExactOptional)
    }

    /// `.nullable()`.
    #[must_use]
    pub fn nullable(&self) -> Self {
        self.wrap(Kind::Nullable)
    }

    /// `.array()`: `z.array(this)`.
    #[must_use]
    pub fn array(&self) -> Self {
        array(self.clone())
    }

    /// `.default(value)`: `value` for `undefined`, not checked.
    #[must_use]
    pub fn default(&self, value: impl Into<JsValue>) -> Self {
        let value = value.into();
        self.wrap(|inner| Kind::Default(inner, value))
    }

    /// `.catch(value)`: `value` where this schema fails.
    #[must_use]
    pub fn catch(&self, value: impl Into<JsValue>) -> Self {
        let value = value.into();
        self.catch_with(move |_context| value.clone())
    }

    /// `.catch((context) => …)`: the value for a failure, from what failed.
    #[must_use]
    pub fn catch_with(
        &self,
        answer: impl Fn(&CatchContext) -> JsValue + Send + Sync + 'static,
    ) -> Self {
        let answer: super::schema::CatchFn = Arc::new(answer);
        self.wrap(|inner| Kind::Catch(inner, answer))
    }

    /// `.transform((value, context) => …)`: what the closure makes of the value; an issue it adds
    /// with `context.add_issue` fails the parse.
    #[must_use]
    pub fn transform(
        &self,
        transform: impl Fn(&JsValue, &mut Context<'_>) -> JsValue + Send + Sync + 'static,
    ) -> Self {
        let last = Self::of(Kind::Transform(Arc::new(transform)));
        self.wrap(|first| Kind::Pipe(first, last))
    }

    /// `.pipe(next)`: `next` on what this schema answered.
    #[must_use]
    pub fn pipe(&self, next: &Self) -> Self {
        let last = next.clone();
        self.wrap(|first| Kind::Pipe(first, last))
    }

    /// `.refine(test, params)`: a `custom` issue where `test` refuses the value.
    #[must_use]
    pub fn refine(
        &self,
        test: impl Fn(&JsValue) -> bool + Send + Sync + 'static,
        params: impl Into<Params>,
    ) -> Self {
        self.checked(CheckKind::Refine(Arc::new(test)), params)
    }

    /// `.superRefine((value, context) => …)`: the issues the closure adds.
    #[must_use]
    pub fn super_refine(
        &self,
        refine: impl Fn(&JsValue, &mut Context<'_>) + Send + Sync + 'static,
    ) -> Self {
        self.checked(CheckKind::SuperRefine(Arc::new(refine)), ())
    }

    /// `.check((context) => …)`: the issues the closure pushes, as written.
    #[must_use]
    pub fn check(&self, check: impl Fn(&mut Context<'_>) + Send + Sync + 'static) -> Self {
        self.with_check(Check::new(CheckKind::Function(Arc::new(check)), None))
    }

    fn numeric(&self) -> bool {
        matches!(self.0.kind, Kind::Number)
    }

    /// `.min(n)`: at least `n` (a number), or a length of at least `n` (a string, an array).
    #[must_use]
    pub fn min(&self, limit: f64) -> Self {
        self.min_with(limit, ())
    }

    /// `.min(n, params)`.
    #[must_use]
    pub fn min_with(&self, limit: f64, params: impl Into<Params>) -> Self {
        let kind = if self.numeric() {
            CheckKind::GreaterThan {
                value: limit,
                inclusive: true,
            }
        } else {
            CheckKind::MinLength(limit)
        };
        self.checked(kind, params)
    }

    /// `.max(n)`: at most `n` (a number), or a length of at most `n` (a string, an array).
    #[must_use]
    pub fn max(&self, limit: f64) -> Self {
        self.max_with(limit, ())
    }

    /// `.max(n, params)`.
    #[must_use]
    pub fn max_with(&self, limit: f64, params: impl Into<Params>) -> Self {
        let kind = if self.numeric() {
            CheckKind::LessThan {
                value: limit,
                inclusive: true,
            }
        } else {
            CheckKind::MaxLength(limit)
        };
        self.checked(kind, params)
    }

    /// `.positive()`: above 0.
    #[must_use]
    pub fn positive(&self) -> Self {
        self.positive_with(())
    }

    /// `.positive(params)`.
    #[must_use]
    pub fn positive_with(&self, params: impl Into<Params>) -> Self {
        let kind = CheckKind::GreaterThan {
            value: 0.0,
            inclusive: false,
        };
        self.checked(kind, params)
    }

    /// `.nonnegative()`: 0 or above.
    #[must_use]
    pub fn nonnegative(&self) -> Self {
        self.nonnegative_with(())
    }

    /// `.nonnegative(params)`.
    #[must_use]
    pub fn nonnegative_with(&self, params: impl Into<Params>) -> Self {
        let kind = CheckKind::GreaterThan {
            value: 0.0,
            inclusive: true,
        };
        self.checked(kind, params)
    }

    /// `.int()`: a safe integer.
    #[must_use]
    pub fn int(&self) -> Self {
        self.checked(CheckKind::SafeInt, ())
    }

    /// `.finite()`: nothing in zod 4, where a number is already finite.
    #[must_use]
    pub fn finite(&self) -> Self {
        self.clone()
    }

    /// `.regex(/source/)`: `source` as the JavaScript literal writes it.
    #[must_use]
    pub fn regex(&self, source: &str) -> Self {
        self.regex_with(source, "", ())
    }

    /// `.regex(/source/flags, params)`.
    #[must_use]
    pub fn regex_with(&self, source: &str, flags: &str, params: impl Into<Params>) -> Self {
        let (literal, regex) = pattern(source, flags);
        self.checked(CheckKind::Regex { literal, regex }, params)
    }

    /// `.shape`: an object schema's members.
    #[must_use]
    pub fn shape(&self) -> Option<&Shape> {
        if let Kind::Object { shape, .. } = &self.0.kind {
            Some(shape)
        } else {
            None
        }
    }

    fn reshaped(&self, change: impl FnOnce(&Shape) -> Shape) -> Self {
        let mut node: Node = (*self.0).clone();
        if let Kind::Object { shape, .. } = &mut node.kind {
            *shape = in_key_order(change(shape));
        }
        Self(Arc::new(node))
    }

    /// `.partial()`: every member `.optional()`.
    #[must_use]
    pub fn partial(&self) -> Self {
        self.reshaped(|shape| {
            shape
                .iter()
                .map(|(key, field)| (key.clone(), field.optional()))
                .collect()
        })
    }

    /// `.required({ key: true, … })`: the members named no longer optional.
    #[must_use]
    pub fn required(&self, keys: &[&str]) -> Self {
        self.reshaped(|shape| {
            shape
                .iter()
                .map(|(key, field)| {
                    let field = if keys.contains(&key.as_str()) {
                        field.wrap(Kind::NonOptional)
                    } else {
                        field.clone()
                    };
                    (key.clone(), field)
                })
                .collect()
        })
    }

    /// `.extend(shape)`: more members, or members replaced in place.
    #[must_use]
    pub fn extend(&self, more: Shape) -> Self {
        self.reshaped(|shape| {
            let mut extended = shape.clone();
            extended.extend(more);
            extended
        })
    }

    /// `.safeParse(value)`: the output (defaults applied, transforms run, unknown keys as the
    /// object kind leaves them), or the error with every issue.
    ///
    /// # Errors
    ///
    /// The issues, when the value does not pass.
    pub fn safe_parse(&self, value: &JsValue) -> Result<JsValue, ZodError> {
        let result = run(self, Payload::of(value.clone()));
        if result.issues.is_empty() {
            Ok(result.value)
        } else {
            Err(ZodError::of(result.issues))
        }
    }

    /// `.parse(value)`: as `safe_parse`; the `Err` is what zod throws.
    ///
    /// # Errors
    ///
    /// The issues, when the value does not pass.
    pub fn parse(&self, value: &JsValue) -> Result<JsValue, ZodError> {
        self.safe_parse(value)
    }
}
