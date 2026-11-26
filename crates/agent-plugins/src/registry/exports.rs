//! The macro a module declares its exports with.

/// Declares the exports of a module of the npm package, at the end of its port:
///
/// ```ignore
/// crate::registry::exports! {
///     "format/path-text" {
///         "childPath" => ok |base: String, name: String| child_path(&base, &name);
///         "parse" => fallible |text: String| parse(&text);
///         "readJson" => async ok |files: Port, path: String, signal: Option<Signal>|
///             read_json(&files, &path, signal.as_ref()).await;
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
/// `async` before the kind marks an export that waits on the host: one the TypeScript declares
/// `async`, or one that takes a port, a callback or an `AbortSignal` ([`Port`], [`Callback`],
/// [`Signal`]). Its body may `.await`; a host runs it with `registry::start` and `resume`, never
/// with `call`, which answers a fault.
///
/// A parameter the TypeScript types `unknown` is a [`JsValue`](crate::js::JsValue), which takes
/// any value, `undefined` included; an `Option` takes `undefined` and `null` as `None`; any other
/// type refuses `undefined` (a fault). It defines `EXPORTS`, the module's
/// [`Module`](crate::registry::Module), for [`MODULES`](crate::registry::MODULES).
///
/// [`Port`]: crate::registry::Port
/// [`Callback`]: crate::registry::Callback
/// [`Signal`]: crate::ports::Signal
macro_rules! exports {
    ($module:literal { $($entries:tt)* }) => {
        $crate::registry::exports!(@munch $module [] $($entries)*);
    };
    (@munch $module:literal [$(($name:literal $how:tt))*]) => {
        /// This module's exports by name (`crate::registry`).
        pub(crate) const EXPORTS: $crate::registry::Module = $crate::registry::Module {
            name: $module,
            functions: &[$($name),*],
            call: exported,
            start: started,
        };

        fn exported(
            function: &str,
            args: &[$crate::js::JsValue],
        ) -> Option<$crate::registry::Answer> {
            // A module whose exports all wait on the host never reads the arguments here.
            let _ = args;
            match function {
                $($name => Some($crate::registry::exports!(@call $how args)),)*
                _ => None,
            }
        }

        fn started(
            function: &str,
            args: &[$crate::js::JsValue],
        ) -> Option<$crate::registry::Started> {
            match function {
                $($name => Some($crate::registry::exports!(@start $how args)),)*
                _ => None,
            }
        }
    };
    (@munch $module:literal [$($done:tt)*]
        $name:literal => async $kind:ident || $body:expr; $($rest:tt)*) => {
        $crate::registry::exports!(@munch $module [$($done)* ($name [async $kind () $body])] $($rest)*);
    };
    (@munch $module:literal [$($done:tt)*]
        $name:literal => async $kind:ident |$($param:ident : $type:ty),* $(,)?| $body:expr;
        $($rest:tt)*) => {
        $crate::registry::exports!(@munch $module
            [$($done)* ($name [async $kind ($($param: $type),*) $body])] $($rest)*);
    };
    (@munch $module:literal [$($done:tt)*] $name:literal => $kind:ident $body:expr; $($rest:tt)*) => {
        $crate::registry::exports!(@munch $module [$($done)* ($name [sync $kind $body])] $($rest)*);
    };
    (@call [sync $kind:ident $body:expr] $args:ident) => {
        $crate::registry::run(|| {
            Ok($crate::registry::$kind($crate::registry::apply($args, $body)?))
        })
    };
    (@call [async $kind:ident $params:tt $body:expr] $args:ident) => {
        $crate::registry::Answer::Fault(
            "it waits on the host: a host starts it (registry::start)".to_owned(),
        )
    };
    (@start [sync $kind:ident $body:expr] $args:ident) => {
        $crate::registry::ready($crate::registry::exports!(@call [sync $kind $body] $args))
    };
    (@start [async $kind:ident ($($param:ident : $type:ty),*) $body:expr] $args:ident) => {
        match $crate::registry::take_args::<($($type,)*)>($args) {
            Ok(($($param,)*)) => {
                let started: $crate::registry::Started =
                    Box::pin(async move { $crate::registry::$kind($body) });
                started
            }
            Err(why) => $crate::registry::ready($crate::registry::Answer::Fault(why)),
        }
    };
}

pub(crate) use exports;
