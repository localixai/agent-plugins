//! A [`JsValue`] read by serde (the wire's spelling, tags read back), and a value as the
//! deserializer of a port's typed parameter.

use std::fmt;

use serde::de::value::{Error, MapAccessDeserializer, MapDeserializer, SeqDeserializer};
use serde::de::{self, Deserialize, Deserializer, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::{JsObject, JsValue};
use crate::js::is_safe_integer;

/// The one-member objects the boundary uses for a value the host keeps on its side
/// (`src/wasm/wire.ts`): a port, a function, a promise.
const HOST_HANDLES: [&str; 3] = ["$port", "$fn", "$promise"];

impl<'de> Deserialize<'de> for JsValue {
    /// Any self-describing value: a unit is `null`, a none `undefined`, and a map that is a wire
    /// tag (`{"$undefined": true}`, `{"$number": name}`) the value it stands for, at every depth.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

/// Every number is a double, as in JavaScript: a wider integer rounds as it does there.
impl<'de> Visitor<'de> for ValueVisitor {
    type Value = JsValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JavaScript value")
    }
    fn visit_bool<E: de::Error>(self, flag: bool) -> Result<JsValue, E> {
        Ok(JsValue::Bool(flag))
    }
    fn visit_i64<E: de::Error>(self, number: i64) -> Result<JsValue, E> {
        Ok(JsValue::Number(number as f64))
    }
    fn visit_u64<E: de::Error>(self, number: u64) -> Result<JsValue, E> {
        Ok(JsValue::Number(number as f64))
    }
    fn visit_f64<E: de::Error>(self, number: f64) -> Result<JsValue, E> {
        Ok(JsValue::Number(number))
    }
    fn visit_str<E: de::Error>(self, text: &str) -> Result<JsValue, E> {
        Ok(JsValue::String(text.to_owned()))
    }
    fn visit_string<E: de::Error>(self, text: String) -> Result<JsValue, E> {
        Ok(JsValue::String(text))
    }
    fn visit_none<E: de::Error>(self) -> Result<JsValue, E> {
        Ok(JsValue::Undefined)
    }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<JsValue, D::Error> {
        JsValue::deserialize(deserializer)
    }
    fn visit_unit<E: de::Error>(self) -> Result<JsValue, E> {
        Ok(JsValue::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<JsValue, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(JsValue::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsValue, A::Error> {
        let mut members = JsObject::new();
        while let Some((key, item)) = map.next_entry::<String, JsValue>()? {
            members.insert(key, item);
        }
        Ok(JsValue::from_tag(&members).unwrap_or(JsValue::Object(members)))
    }
}

/// A number to the visitor as serde's integer types take one: an integer a double holds exactly
/// as an integer (`-0` excepted, which only a double holds), any other as a double.
fn visit_number<'de, V: Visitor<'de>>(number: f64, visitor: V) -> Result<V::Value, Error> {
    let negative_zero = number == 0.0 && number.is_sign_negative();
    if !is_safe_integer(number) || negative_zero {
        return visitor.visit_f64(number);
    }
    if number >= 0.0 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "integral, within 2^53 and not negative"
        )]
        let whole = number as u64;
        return visitor.visit_u64(whole);
    }
    #[expect(clippy::cast_possible_truncation, reason = "integral and within 2^53")]
    let whole = number as i64;
    visitor.visit_i64(whole)
}

impl<'de> Deserializer<'de> for JsValue {
    type Error = Error;

    /// `undefined` is a none and `null` a unit, so a `JsValue` reads each back as itself.
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self {
            Self::Undefined => visitor.visit_none(),
            Self::Null => visitor.visit_unit(),
            Self::Bool(flag) => visitor.visit_bool(flag),
            Self::Number(number) => visit_number(number, visitor),
            Self::String(text) => visitor.visit_string(text),
            Self::Array(items) => {
                let mut seq = SeqDeserializer::new(items.into_iter());
                let value = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(value)
            }
            Self::Object(members) => {
                let mut map = MapDeserializer::new(members.into_iter());
                let value = visitor.visit_map(&mut map)?;
                map.end()?;
                Ok(value)
            }
        }
    }

    /// `undefined` and `null` are `None`, as an optional parameter takes either.
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.is_nullish() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    /// A string is a unit variant; an object of one member a variant with data.
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self {
            Self::String(text) => visitor.visit_enum(text.into_deserializer()),
            Self::Object(members) if members.len() == 1 => visitor.visit_enum(
                MapAccessDeserializer::new(MapDeserializer::new(members.into_iter())),
            ),
            other @ (Self::Undefined
            | Self::Null
            | Self::Bool(_)
            | Self::Number(_)
            | Self::Array(_)
            | Self::Object(_)) => Err(de::Error::custom(format!(
                "expected a variant, found {}",
                other.type_of()
            ))),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    /// A host value kept on the host's side (`{"$port": n}`, `{"$fn": n}`: an object that holds a
    /// function crosses as one) is refused where a struct of data is read, unless the struct is
    /// that handle: read as data, it would be an object with none of its fields, and every
    /// optional one would read as absent.
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        if let Self::Object(members) = &self
            && let [(key, _)] = members.iter().collect::<Vec<_>>()[..]
            && HOST_HANDLES.contains(&key.as_str())
            && !fields.contains(&key.as_str())
        {
            return Err(de::Error::custom(format!(
                "{name} is data, but the host passed a value it keeps ({key}): an object that \
                 holds a function crosses as a port, so pass the fields {name} reads"
            )));
        }
        self.deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map identifier ignored_any
    }
}

impl IntoDeserializer<'_, Error> for JsValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}
