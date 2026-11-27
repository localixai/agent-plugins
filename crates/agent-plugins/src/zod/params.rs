//! What a builder or a check takes besides its value: zod's `params` (`{ error }`, `{ message }`,
//! a function of the issue, a refinement's `path` and `abort`), and the shape of an object.

use std::sync::Arc;

use indexmap::IndexMap;

use super::Schema;
use super::issue::RawIssue;
use crate::js::JsValue;

/// An error map: zod's `error` option as a function, the message of an issue its schema or check
/// raised, or `None` to leave it to the English default (zod's `unwrapMessage` of `undefined`).
pub type ErrorMap = Arc<dyn Fn(&RawIssue) -> Option<String> + Send + Sync>;

/// The members of an object schema, in the order they were written: zod's shape.
pub type Shape = IndexMap<String, Schema>;

/// zod's params, as `util.normalizeParams` reads them: a message (`'…'`, `{ error: '…' }` and
/// `{ message: '…' }` are one thing), or a function of the raw issue, and for a refinement where
/// its issue is reported (`path`) and whether it stops the checks after it (`abort`).
#[derive(Clone, Default)]
pub struct Params {
    error: Option<ErrorMap>,
    path: Vec<JsValue>,
    abort: Option<bool>,
}

impl Params {
    /// `{ path: [...] }`: where a refinement reports its issue, below the value it refines.
    #[must_use]
    pub fn path<I: Into<JsValue>>(mut self, path: impl IntoIterator<Item = I>) -> Self {
        self.path = path.into_iter().map(Into::into).collect();
        self
    }

    /// `{ abort: true }`: a failed check stops the checks after it.
    #[must_use]
    pub const fn abort(mut self, abort: bool) -> Self {
        self.abort = Some(abort);
        self
    }

    pub(crate) fn error_map(&self) -> Option<ErrorMap> {
        self.error.clone()
    }

    pub(crate) fn refine_path(&self) -> Vec<JsValue> {
        self.path.clone()
    }

    /// `abort`, or what the builder defaults it to (`z.custom` aborts, a refinement does not).
    pub(crate) fn aborts(&self, default: bool) -> bool {
        self.abort.unwrap_or(default)
    }
}

impl std::fmt::Debug for Params {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Params")
            .field("error", &self.error.is_some())
            .field("path", &self.path)
            .field("abort", &self.abort)
            .finish()
    }
}

/// No params: `.refine(test, ())` is TypeScript's `.refine(test)`.
impl From<()> for Params {
    fn from((): ()) -> Self {
        Self::default()
    }
}

/// `{ error: '…' }` or `{ message: '…' }`: the issue's message.
impl From<&str> for Params {
    fn from(message: &str) -> Self {
        Self::from(message.to_owned())
    }
}

/// `{ error: message }` with a message built at run time.
impl From<String> for Params {
    fn from(message: String) -> Self {
        Self {
            error: Some(Arc::new(move |_issue: &RawIssue| Some(message.clone()))),
            ..Self::default()
        }
    }
}

/// `{ error: (issue) => … }`: the message as a function of the raw issue (its `input`, its
/// `code`), the error option the package passes where the message depends on the value.
#[must_use]
pub fn error_fn(map: impl Fn(&RawIssue) -> String + Send + Sync + 'static) -> Params {
    Params {
        error: Some(Arc::new(move |issue: &RawIssue| Some(map(issue)))),
        ..Params::default()
    }
}

/// The literal of a shape, `{ key: schema, ...spread }`: each `"key" => schema`, or `..shape` to
/// spread another shape's members, where a key written twice keeps its first place and takes the
/// last schema, as a spread does in an object literal.
///
/// ```ignore
/// zod::object(zod::fields! { "name" => zod::string(), ..common(), "model" => text().optional() })
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __zod_fields {
    (@into $shape:ident) => {};
    (@into $shape:ident .. $spread:expr $(, $($rest:tt)*)?) => {
        $shape.extend($spread);
        $($crate::__zod_fields!(@into $shape $($rest)*);)?
    };
    (@into $shape:ident $key:literal => $schema:expr $(, $($rest:tt)*)?) => {
        $shape.insert(::std::string::String::from($key), $schema);
        $($crate::__zod_fields!(@into $shape $($rest)*);)?
    };
    ($($body:tt)*) => {{
        let mut shape = $crate::zod::Shape::new();
        $crate::__zod_fields!(@into shape $($body)*);
        shape
    }};
}
