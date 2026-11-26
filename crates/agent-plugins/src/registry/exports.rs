//! The macro a module declares its exports with.

/// Declares the exports of a module of the npm package, at the end of its port:
///
/// ```ignore
/// crate::registry::exports! {
///     "format/path-text" {
///         "childPath" => ok |base: String, name: String| child_path(&base, &name);
///         "parse" => fallible |text: String| parse(&text);
///     }
/// }
/// ```
///
/// Each export is its exported name, how its result is answered, and a closure that calls the
/// port (its parameters are decoded from the call's arguments in order: a missing one is
/// `undefined`, an extra one is ignored, as JavaScript does; a constant is a closure with none,
/// `|| NAME`):
///
/// - `ok`: the value; `None` is `undefined`;
/// - `nullable`: the value; `None` is `null` (the TypeScript returns `null`);
/// - `fallible`: a `Result` whose `Err` is what the TypeScript throws.
///
/// A parameter the TypeScript types `unknown` is a [`JsValue`](crate::js::JsValue), which takes
/// any value, `undefined` included; an `Option` takes `undefined` and `null` as `None`; any other
/// type refuses `undefined` (a fault). It defines `EXPORTS`, the module's
/// [`Module`](crate::registry::Module), for [`MODULES`](crate::registry::MODULES).
macro_rules! exports {
    ($module:literal {
        $($name:literal => $kind:ident $body:expr;)*
    }) => {
        /// This module's exports by name (`crate::registry`).
        pub(crate) const EXPORTS: $crate::registry::Module = $crate::registry::Module {
            name: $module,
            functions: &[$($name),*],
            call: exported,
        };

        fn exported(
            function: &str,
            args: &[$crate::js::JsValue],
        ) -> Option<$crate::registry::Answer> {
            match function {
                $($name => Some($crate::registry::run(|| {
                    Ok($crate::registry::$kind($crate::registry::apply(args, $body)?))
                })),)*
                _ => None,
            }
        }
    };
}

pub(crate) use exports;
