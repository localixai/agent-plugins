//! Ported from `src/format/defined.ts`; only the core uses it now, so it has no wrapper.
//!
//! An object without its `undefined` values, so it can be spread over another without erasing
//! what the other holds. `null` is a value, and stays.

use crate::js::JsObject;

/// The members of `value` that are not `undefined`, in their order.
#[must_use]
pub fn defined(value: JsObject) -> JsObject {
    value
        .into_iter()
        .filter(|(_, item)| !item.is_undefined())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::JsValue;

    #[test]
    fn undefined_goes_and_null_stays() {
        let value = JsObject::from([
            ("a".to_owned(), JsValue::Undefined),
            ("b".to_owned(), JsValue::Null),
            ("c".to_owned(), JsValue::Number(f64::NAN)),
        ]);
        let kept: Vec<String> = defined(value).into_keys().collect();
        assert_eq!(kept, ["b", "c"]);
    }
}
