//! Results with methods: what the TypeScript returns as an object whose members are functions (a
//! hook engine, a plugin host) crosses as a handle, `{"$handle": h, "sync": [names], "async":
//! [names]}`. The core keeps the object under `h`; the host calls a method by the name
//! `<h>#<method>` through `registry::start` and `resume`, as it calls an export, and gives the
//! handle back with `registry::release` when it no longer holds it. A sync method is one the
//! TypeScript returns a value from, an async one a promise; the host's wrapper builds its object
//! with the same kinds.
//!
//! A type declares its methods with `methods!` (below), the way a module declares its exports.
//!
//! A handle made with [`Handle::own`] crosses as `{"$own": h, …}`; given back (an argument of an
//! export or a method) it crosses as `{"$handle": h}` and is the core's own object again, an [`Own`]: so a module whose TypeScript took an object it
//! made earlier (a session's core) takes it by its number, and nothing is copied or asked.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{SerializeMap, Serializer};

use super::task::Started;
use crate::js::JsValue;

/// An object the host calls methods of, by name (`methods!` implements it).
pub trait Methods: Send + Sync + 'static {
    /// The names of the methods that answer at once (a value in the TypeScript).
    fn sync_methods(&self) -> &'static [&'static str];
    /// The names of the methods that wait on the host (a promise in the TypeScript).
    fn async_methods(&self) -> &'static [&'static str];
    /// Starts `method` with `args`, or `None` for a name it does not have.
    fn start(self: Arc<Self>, method: &str, args: &[JsValue]) -> Option<Started>;
    /// The object as `Any`, for an [`Own`] to find its type again.
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

/// A result the host holds by a handle: it serializes as `{"$handle": h, …}` and the core keeps
/// the object under `h` until the host releases it.
#[derive(Clone)]
pub struct Handle(Arc<dyn Methods>, bool);

impl Handle {
    /// The object as a result.
    pub fn new(object: impl Methods) -> Self {
        Self(Arc::new(object), false)
    }

    /// The object as a result the host may give back, as an [`Own`]: it crosses as `{"$own": h,
    /// …}`, so the host remembers its object as the core's. Any other handle given back is the
    /// host's value (a port), as the core reads it where a port is expected.
    pub fn own(object: impl Methods) -> Self {
        Self(Arc::new(object), true)
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Handle")
            .field("sync", &self.0.sync_methods())
            .field("async", &self.0.async_methods())
            .finish()
    }
}

#[derive(Default)]
struct Handles {
    next: u32,
    held: BTreeMap<u32, Arc<dyn Methods>>,
}

thread_local! {
    static HANDLES: Mutex<Handles> = Mutex::new(Handles::default());
}

fn with_handles<R>(body: impl FnOnce(&mut Handles) -> R) -> R {
    HANDLES.with(|handles| body(&mut handles.lock().unwrap_or_else(PoisonError::into_inner)))
}

/// Keeps `object` under a new number, which it answers.
fn keep(object: &Arc<dyn Methods>) -> u32 {
    with_handles(|handles| {
        let number = handles.next;
        handles.next = handles.next.wrapping_add(1);
        handles.held.insert(number, Arc::clone(object));
        number
    })
}

/// A function of the core's that the host calls later, such as a timer's `fire`: an object whose
/// one sync method is `call`, crossing as `{"$call": h}`, which the host makes a function that
/// calls `<h>#call` (`src/wasm/wire.ts`). It is held as a handle is, until the host releases it.
#[derive(Clone)]
pub struct Function(Arc<dyn Methods>);

impl Function {
    /// The object as a function; its sync method `call` is what calling it does.
    pub fn new(object: impl Methods) -> Self {
        Self(Arc::new(object))
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Function")
    }
}

impl Serialize for Function {
    /// Keeps the object under a new number, which it writes as `{"$call": h}`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("$call", &keep(&self.0))?;
        map.end()
    }
}

impl Serialize for Handle {
    /// Keeps the object under a new number, which it writes with the names of its methods.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let number = keep(&self.0);
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry(if self.1 { "$own" } else { "$handle" }, &number)?;
        map.serialize_entry("sync", self.0.sync_methods())?;
        map.serialize_entry("async", self.0.async_methods())?;
        map.end()
    }
}

/// Starts `method` of the object held under `handle`; `None` when there is no such handle or
/// method.
pub(crate) fn start(handle: u32, method: &str, args: &[JsValue]) -> Option<Started> {
    let object = with_handles(|handles| handles.held.get(&handle).cloned())?;
    object.start(method, args)
}

/// Forgets the object held under `handle` (a call of one of its methods already started keeps
/// it until it ends); whether there was one.
pub fn release(handle: u32) -> bool {
    with_handles(|handles| handles.held.remove(&handle).is_some())
}

/// A handle of the core's the host gave back, `{"$handle": h}`: the object held under `h`, of type
/// `T`. Reading it fails when no object is held under `h` or the one held is another type.
pub struct Own<T>(pub Arc<T>);

impl<T> std::ops::Deref for Own<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::fmt::Debug for Own<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Own")
    }
}

/// The tag a handle given back crosses as.
#[derive(serde::Deserialize)]
struct OwnTag {
    #[serde(rename = "$handle")]
    handle: u32,
}

impl<'de, T: Methods> Deserialize<'de> for Own<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let OwnTag { handle } = OwnTag::deserialize(deserializer)?;
        let held = with_handles(|handles| handles.held.get(&handle).cloned());
        held.and_then(|object| object.as_any().downcast::<T>().ok())
            .map(Own)
            .ok_or_else(|| {
                de::Error::custom(format!(
                    "handle {handle} is not a {} the core holds",
                    std::any::type_name::<T>()
                ))
            })
    }
}

/// How many objects are held, for the tests.
#[cfg(test)]
pub(crate) fn held() -> usize {
    with_handles(|handles| handles.held.len())
}

/// Declares a type's methods for its [`Handle`], as `exports!` declares a module's exports; the
/// first parameter is the object (an `Arc<Self>`):
///
/// ```ignore
/// crate::registry::methods! {
///     Counter {
///         "add" => ok |this, by: f64| this.add(by);
///         "settle" => async ok |this, path: String| this.settle(&path).await;
///         "home" => sync async ok |this| this.locations.home_dir().await;
///     }
/// }
/// ```
///
/// `sync async` marks a method the TypeScript answers at once that still asks the host (a
/// `Locations.homeDir` that delegates to the host's own): the host lists it as sync and answers
/// each of its needs as it comes, as a sync wrapper does (`src/wasm/drive.ts`).
macro_rules! methods {
    ($type:ty { $($entries:tt)* }) => {
        $crate::registry::methods!(@munch $type [] [] [] $($entries)*);
    };
    (@munch $type:ty [$($sync:literal)*] [$($async:literal)*] [$(($name:literal $this:ident $how:tt))*]) => {
        impl $crate::registry::Methods for $type {
            fn sync_methods(&self) -> &'static [&'static str] {
                &[$($sync),*]
            }

            fn async_methods(&self) -> &'static [&'static str] {
                &[$($async),*]
            }

            fn start(
                self: ::std::sync::Arc<Self>,
                method: &str,
                args: &[$crate::js::JsValue],
            ) -> Option<$crate::registry::Started> {
                // A type with no methods (a session's core, only given back) reads no argument.
                let _: &[$crate::js::JsValue] = args;
                match method {
                    $($name => {
                        let $this = self;
                        Some($crate::registry::exports!(@start $how args))
                    })*
                    _ => None,
                }
            }

            fn as_any(
                self: ::std::sync::Arc<Self>,
            ) -> ::std::sync::Arc<dyn ::std::any::Any + Send + Sync> {
                self
            }
        }
    };
    (@munch $type:ty [$($sync:literal)*] [$($async:literal)*] [$($done:tt)*]
        $name:literal => sync async $kind:ident |$this:ident $(, $param:ident : $ptype:ty)* $(,)?| $body:expr;
        $($rest:tt)*) => {
        $crate::registry::methods!(@munch $type [$($sync)* $name] [$($async)*]
            [$($done)* ($name $this [async $kind ($($param: $ptype),*) $body])] $($rest)*);
    };
    (@munch $type:ty [$($sync:literal)*] [$($async:literal)*] [$($done:tt)*]
        $name:literal => async $kind:ident |$this:ident $(, $param:ident : $ptype:ty)* $(,)?| $body:expr;
        $($rest:tt)*) => {
        $crate::registry::methods!(@munch $type [$($sync)*] [$($async)* $name]
            [$($done)* ($name $this [async $kind ($($param: $ptype),*) $body])] $($rest)*);
    };
    (@munch $type:ty [$($sync:literal)*] [$($async:literal)*] [$($done:tt)*]
        $name:literal => $kind:ident |$this:ident $(, $param:ident : $ptype:ty)* $(,)?| $body:expr;
        $($rest:tt)*) => {
        $crate::registry::methods!(@munch $type [$($sync)* $name] [$($async)*]
            [$($done)* ($name $this [sync $kind (move |$($param: $ptype),*| $body)])] $($rest)*);
    };
}

pub(crate) use methods;
