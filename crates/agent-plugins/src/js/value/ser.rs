//! A [`JsValue`] written by serde, and any serializable value read into one.

use serde::de::value::Error;
use serde::ser::{
    self, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

use super::{JsObject, JsValue, number_tag};
use crate::js::{is_safe_integer, number_to_string};

impl Serialize for JsValue {
    /// The wire's spelling (the module comment of [`crate::js::value`]).
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Undefined => tag(serializer, "$undefined", &true),
            Self::Null => serializer.serialize_unit(),
            Self::Bool(flag) => serializer.serialize_bool(*flag),
            Self::Number(number) => serialize_number(number, serializer),
            Self::String(text) => serializer.serialize_str(text),
            Self::Array(items) => serializer.collect_seq(items),
            Self::Object(members) => serializer.collect_map(members),
        }
    }
}

fn tag<S: ser::Serializer, V: Serialize + ?Sized>(
    serializer: S,
    key: &str,
    value: &V,
) -> Result<S::Ok, S::Error> {
    let mut map = serializer.serialize_map(Some(1))?;
    map.serialize_entry(key, value)?;
    map.end()
}

/// A number in the wire's spelling: `NaN`, the infinities and `-0` tagged, an integer a double
/// holds exactly as an integer (`1`, not `1.0`), any other as a double. For a field the
/// TypeScript types `number` that may hold one of the four:
/// `#[serde(serialize_with = "crate::js::value::serialize_number")]`.
///
/// # Errors
///
/// What the serializer refuses.
pub fn serialize_number<S: ser::Serializer>(
    number: &f64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if let Some(name) = number_tag(*number) {
        return tag(serializer, "$number", name);
    }
    if is_safe_integer(*number) {
        #[expect(clippy::cast_possible_truncation, reason = "integral and within 2^53")]
        return serializer.serialize_i64(*number as i64);
    }
    serializer.serialize_f64(*number)
}

/// Any serializable value as a [`JsValue`]: a struct is an object of its fields, `None` and `()`
/// are `null` (as `serde_json` writes them), every number a double, a wire tag its value.
///
/// # Errors
///
/// A map whose key is no string or number.
pub fn to_js_value<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, Error> {
    value.serialize(ValueSerializer)
}

struct ValueSerializer;

/// The items of an array being written.
struct Items(Vec<JsValue>);

/// The members of an object being written, and the key of the member whose value comes next.
#[derive(Default)]
struct Members {
    members: JsObject,
    key: Option<String>,
}

/// A variant with data, written as `{ variant: data }` (serde's external tagging).
struct Variant<T> {
    name: &'static str,
    data: T,
}

fn variant(name: &str, data: JsValue) -> JsValue {
    JsValue::Object(JsObject::from([(name.to_owned(), data)]))
}

/// Every number is a double, as in JavaScript: a wider integer rounds as it does there.
impl ser::Serializer for ValueSerializer {
    type Ok = JsValue;
    type Error = Error;
    type SerializeSeq = Items;
    type SerializeTuple = Items;
    type SerializeTupleStruct = Items;
    type SerializeTupleVariant = Variant<Items>;
    type SerializeMap = Members;
    type SerializeStruct = Members;
    type SerializeStructVariant = Variant<Members>;

    fn serialize_bool(self, flag: bool) -> Result<JsValue, Error> {
        Ok(JsValue::Bool(flag))
    }
    fn serialize_i8(self, number: i8) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_i16(self, number: i16) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_i32(self, number: i32) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_i64(self, number: i64) -> Result<JsValue, Error> {
        Ok(JsValue::Number(number as f64))
    }
    fn serialize_u8(self, number: u8) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_u16(self, number: u16) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_u32(self, number: u32) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_u64(self, number: u64) -> Result<JsValue, Error> {
        Ok(JsValue::Number(number as f64))
    }
    fn serialize_f32(self, number: f32) -> Result<JsValue, Error> {
        Ok(JsValue::Number(f64::from(number)))
    }
    fn serialize_f64(self, number: f64) -> Result<JsValue, Error> {
        Ok(JsValue::Number(number))
    }
    fn serialize_char(self, character: char) -> Result<JsValue, Error> {
        Ok(JsValue::String(character.to_string()))
    }
    fn serialize_str(self, text: &str) -> Result<JsValue, Error> {
        Ok(JsValue::String(text.to_owned()))
    }
    fn serialize_bytes(self, bytes: &[u8]) -> Result<JsValue, Error> {
        let items = bytes.iter().map(|byte| JsValue::Number(f64::from(*byte)));
        Ok(JsValue::Array(items.collect()))
    }
    fn serialize_none(self) -> Result<JsValue, Error> {
        Ok(JsValue::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<JsValue, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<JsValue, Error> {
        Ok(JsValue::Null)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<JsValue, Error> {
        Ok(JsValue::Null)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        name: &'static str,
    ) -> Result<JsValue, Error> {
        Ok(JsValue::String(name.to_owned()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<JsValue, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        name: &'static str,
        value: &T,
    ) -> Result<JsValue, Error> {
        Ok(variant(name, to_js_value(value)?))
    }
    fn serialize_seq(self, length: Option<usize>) -> Result<Items, Error> {
        Ok(Items(Vec::with_capacity(length.unwrap_or(0))))
    }
    fn serialize_tuple(self, length: usize) -> Result<Items, Error> {
        Ok(Items(Vec::with_capacity(length)))
    }
    fn serialize_tuple_struct(self, _name: &'static str, length: usize) -> Result<Items, Error> {
        Ok(Items(Vec::with_capacity(length)))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        name: &'static str,
        length: usize,
    ) -> Result<Variant<Items>, Error> {
        let data = Items(Vec::with_capacity(length));
        Ok(Variant { name, data })
    }
    fn serialize_map(self, _length: Option<usize>) -> Result<Members, Error> {
        Ok(Members::default())
    }
    fn serialize_struct(self, _name: &'static str, _length: usize) -> Result<Members, Error> {
        Ok(Members::default())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        name: &'static str,
        _length: usize,
    ) -> Result<Variant<Members>, Error> {
        let data = Members::default();
        Ok(Variant { name, data })
    }
}

impl SerializeSeq for Items {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.0.push(to_js_value(value)?);
        Ok(())
    }
    fn end(self) -> Result<JsValue, Error> {
        Ok(JsValue::Array(self.0))
    }
}

impl SerializeTuple for Items {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<JsValue, Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for Items {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<JsValue, Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleVariant for Variant<Items> {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(&mut self.data, value)
    }
    fn end(self) -> Result<JsValue, Error> {
        Ok(variant(self.name, JsValue::Array(self.data.0)))
    }
}

/// A map key as an object's key: a string, or a number or boolean as its text.
fn key_text(key: JsValue) -> Result<String, Error> {
    match key {
        JsValue::String(text) => Ok(text),
        JsValue::Number(number) => Ok(number_to_string(number)),
        JsValue::Bool(flag) => Ok(flag.to_string()),
        JsValue::Undefined | JsValue::Null | JsValue::Array(_) | JsValue::Object(_) => Err(
            ser::Error::custom(format!("an object key cannot be {}", key.type_of())),
        ),
    }
}

impl SerializeMap for Members {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(key_text(to_js_value(key)?)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.key.take().unwrap_or_default();
        self.members.insert(key, to_js_value(value)?);
        Ok(())
    }
    /// A tag (`JsValue`'s own spelling of `undefined` and the four numbers) is read as its value.
    fn end(self) -> Result<JsValue, Error> {
        Ok(JsValue::from_tag(&self.members).unwrap_or(JsValue::Object(self.members)))
    }
}

impl SerializeStruct for Members {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.members.insert(key.to_owned(), to_js_value(value)?);
        Ok(())
    }
    fn end(self) -> Result<JsValue, Error> {
        Ok(JsValue::Object(self.members))
    }
}

impl SerializeStructVariant for Variant<Members> {
    type Ok = JsValue;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        SerializeStruct::serialize_field(&mut self.data, key, value)
    }
    fn end(self) -> Result<JsValue, Error> {
        Ok(variant(self.name, JsValue::Object(self.data.members)))
    }
}
