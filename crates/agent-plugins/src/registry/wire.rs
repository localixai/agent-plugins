//! The values that cross the boundary: arguments decoded into a port's parameters, results and
//! errors encoded into an [`Answer`], with the tags the npm package's wrapper writes for what
//! JSON has no spelling for (`src/wasm/wire.ts`).

use std::fmt::Display;

use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};

use crate::js::value::{JsValue, to_js_value, write_json_string};

/// What a call answers (the module comment of [`crate::registry`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// The function's result.
    Result(JsValue),
    /// The message of what the function throws.
    Throws(String),
    /// A bug on one side of the boundary: an unknown name, arguments that do not decode.
    Fault(String),
}

impl Answer {
    /// The answer as the JSON text of the object the wrapper reads: `{"result": value}` in the
    /// wire's spelling, `{"throws": message}` or `{"fault": why}`.
    #[must_use]
    pub fn to_json_text(&self) -> String {
        let (key, text) = match self {
            Self::Result(value) => return format!(r#"{{"result":{}}}"#, value.to_json_text()),
            Self::Throws(message) => ("throws", message),
            Self::Fault(why) => ("fault", why),
        };
        let mut out = format!(r#"{{"{key}":"#);
        write_json_string(text, &mut out);
        out.push('}');
        out
    }
}

/// A JavaScript `Set` as the wrapper tags it (`{"$set": [...]}`), in the order it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Set<T> {
    /// The members.
    #[serde(rename = "$set")]
    pub members: Vec<T>,
}

/// A JavaScript `Map` as the wrapper tags it (`{"$map": [[key, value]…]}`), in the order it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Map<K, V> {
    /// The entries.
    #[serde(rename = "$map")]
    pub entries: Vec<(K, V)>,
}

/// A `RegExp` as the wrapper tags it (`{"$regexp": source, "flags": flags}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegExp {
    /// The pattern's source.
    #[serde(rename = "$regexp")]
    pub source: String,
    /// Its flags.
    pub flags: String,
}

/// A `Uint8Array` as the wrapper tags it (`{"$bytes": base64}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

/// One base64 character's value, `None` for a character the alphabet lacks.
const fn sextet(character: u8) -> Option<u8> {
    match character {
        b'A'..=b'Z' => Some(character - b'A'),
        b'a'..=b'z' => Some(character - b'a' + 26),
        b'0'..=b'9' => Some(character - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Tagged {
            #[serde(rename = "$bytes")]
            base64: String,
        }
        let Tagged { base64 } = Tagged::deserialize(deserializer)?;
        let mut bytes = Vec::new();
        let (mut buffer, mut bits) = (0u32, 0u32);
        for character in base64.bytes().filter(|byte| *byte != b'=') {
            let value = sextet(character).ok_or_else(|| {
                serde::de::Error::custom(
                    "the bytes are not base64: a character outside its alphabet",
                )
            })?;
            buffer = (buffer << 6) | u32::from(value);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                bytes.push(u8::try_from((buffer >> bits) & 0xff).unwrap_or_default());
            }
        }
        Ok(Self(bytes))
    }
}

/// The base64 alphabet, by each character's value.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

impl Serialize for Bytes {
    /// `{"$bytes": base64}`, padded, as the wrapper reads it.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut base64 = String::new();
        for chunk in self.0.chunks(3) {
            let group = chunk.iter().enumerate().fold(0u32, |group, (at, byte)| {
                group | u32::from(*byte) << (16 - 8 * at)
            });
            for at in 0..4 {
                let sextet = usize::try_from((group >> (18 - 6 * at)) & 0x3f).unwrap_or_default();
                let character = ALPHABET.get(sextet).copied().unwrap_or(b'=');
                base64.push(if at <= chunk.len() {
                    char::from(character)
                } else {
                    '='
                });
            }
        }
        let mut map = serializer.serialize_map(Some(1))?;
        serde::ser::SerializeMap::serialize_entry(&mut map, "$bytes", &base64)?;
        serde::ser::SerializeMap::end(map)
    }
}

/// One parameter, decoded from the argument at its place.
pub trait Param: Sized {
    /// The argument at `index` (missing: `undefined`) as this parameter.
    ///
    /// # Errors
    ///
    /// The argument does not decode as the parameter's type.
    fn param(args: &[JsValue], index: usize) -> Result<Self, String>;
}

impl<T: DeserializeOwned> Param for T {
    /// The argument decoded as `T`, with JavaScript's values as they are: a [`JsValue`]
    /// parameter takes any (`undefined` too), an `Option` takes `undefined` and `null` as `None`,
    /// any other refuses `undefined`.
    fn param(args: &[JsValue], index: usize) -> Result<Self, String> {
        let raw = args.get(index).cloned().unwrap_or_default();
        T::deserialize(raw).map_err(|error| format!("argument {index} does not decode: {error}"))
    }
}

/// A TypeScript rest parameter (`...events: HookEvent[]`): the call's arguments from this
/// parameter's place on, each decoded as `T`. It is an export's last parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rest<T>(pub Vec<T>);

impl<T: DeserializeOwned> Param for Rest<T> {
    fn param(args: &[JsValue], index: usize) -> Result<Self, String> {
        let rest = args.get(index..).unwrap_or_default();
        rest.iter()
            .zip(index..)
            .map(|(arg, at)| {
                T::deserialize(arg.clone())
                    .map_err(|error| format!("argument {at} does not decode: {error}"))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

/// A parameter list, decoded from a call's arguments.
pub trait Params: Sized {
    /// The arguments as the list.
    ///
    /// # Errors
    ///
    /// An argument does not decode.
    fn params(args: &[JsValue]) -> Result<Self, String>;
}

macro_rules! params {
    ($($name:ident $index:tt),*) => {
        impl<$($name: Param),*> Params for ($($name,)*) {
            #[allow(unused_variables, clippy::allow_attributes, reason = "the empty list reads no argument")]
            fn params(args: &[JsValue]) -> Result<Self, String> {
                Ok(($($name::param(args, $index)?,)*))
            }
        }
    };
}

params!();
params!(A 0);
params!(A 0, B 1);
params!(A 0, B 1, C 2);
params!(A 0, B 1, C 2, D 3);
params!(A 0, B 1, C 2, D 3, E 4);
params!(A 0, B 1, C 2, D 3, E 4, F 5);
params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);

/// An export's body: a closure over decoded parameters (none, for a constant), called with the
/// arguments of a call.
pub trait Handler<P> {
    /// What the body answers.
    type Output;

    /// Decodes the arguments for the body's parameters and calls it.
    ///
    /// # Errors
    ///
    /// An argument does not decode.
    fn call(self, args: &[JsValue]) -> Result<Self::Output, String>;
}

macro_rules! handlers {
    ($($name:ident $index:tt),*) => {
        impl<Body, Out, $($name: Param),*> Handler<($($name,)*)> for Body
        where
            Body: FnOnce($($name),*) -> Out,
        {
            type Output = Out;

            #[allow(unused_variables, clippy::allow_attributes, reason = "a constant reads no argument")]
            fn call(self, args: &[JsValue]) -> Result<Out, String> {
                Ok(self($($name::param(args, $index)?),*))
            }
        }
    };
}

handlers!();
handlers!(A 0);
handlers!(A 0, B 1);
handlers!(A 0, B 1, C 2);
handlers!(A 0, B 1, C 2, D 3);
handlers!(A 0, B 1, C 2, D 3, E 4);
handlers!(A 0, B 1, C 2, D 3, E 4, F 5);
handlers!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
handlers!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);

/// The body of an export called with a call's arguments (for `exports!`(crate::registry::exports)).
///
/// # Errors
///
/// An argument does not decode.
pub fn apply<P, B: Handler<P>>(args: &[JsValue], body: B) -> Result<B::Output, String> {
    body.call(args)
}

/// The arguments of a call as a parameter list (for `exports!`(crate::registry::exports)).
///
/// # Errors
///
/// An argument does not decode.
pub fn take_args<P: Params>(args: &[JsValue]) -> Result<P, String> {
    P::params(args)
}

/// An answer already given, as a task's future (a sync export started as a task).
pub fn ready(answer: Answer) -> super::Started {
    Box::pin(std::future::ready(answer))
}

/// The answer of a call's body, a decoding failure as a fault.
pub fn run(body: impl FnOnce() -> Result<Answer, String>) -> Answer {
    body().unwrap_or_else(Answer::Fault)
}

fn encode<R: Serialize>(result: &R) -> Result<JsValue, String> {
    to_js_value(result).map_err(|error| format!("the result does not encode: {error}"))
}

/// A result; `None` (and so `null` at the top) is `undefined`.
pub fn ok<R: Serialize>(result: R) -> Answer {
    match encode(&result) {
        Ok(JsValue::Null) => Answer::Result(JsValue::Undefined),
        Ok(value) => Answer::Result(value),
        Err(why) => Answer::Fault(why),
    }
}

/// A result whose `None` is `null`, for a function the TypeScript types `… | null`.
pub fn nullable<R: Serialize>(result: R) -> Answer {
    encode(&result).map_or_else(Answer::Fault, Answer::Result)
}

/// A result or what the TypeScript throws.
pub fn fallible<R: Serialize, E: Display>(result: Result<R, E>) -> Answer {
    match result {
        Ok(value) => ok(value),
        Err(error) => Answer::Throws(error.to_string()),
    }
}

/// An order as JavaScript's comparators answer it: `-1`, `0` or `1`.
#[must_use]
pub const fn sign(order: std::cmp::Ordering) -> i8 {
    match order {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
