//! JavaScript's `array.join(separator)`: each item as `String(item)` would write it, `undefined`
//! and `null` as nothing, a nested array joined with `,` and an object as `[object Object]`.

use super::{JsValue, number_to_string};

/// `items.join(separator)`.
#[must_use]
pub fn array_join(items: &[JsValue], separator: &str) -> String {
    items
        .iter()
        .map(item_text)
        .collect::<Vec<String>>()
        .join(separator)
}

/// What `join` writes for one item.
fn item_text(item: &JsValue) -> String {
    match item {
        JsValue::Undefined | JsValue::Null => String::new(),
        JsValue::Bool(flag) => flag.to_string(),
        JsValue::Number(number) => number_to_string(*number),
        JsValue::String(text) => text.clone(),
        JsValue::Array(nested) => array_join(nested, ","),
        JsValue::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_each_item_as_javascript_writes_it() {
        let items = vec![
            JsValue::String("a".to_owned()),
            JsValue::Null,
            JsValue::Number(1.5),
            JsValue::Bool(true),
            JsValue::Array(vec![JsValue::Number(1.0), JsValue::Undefined]),
            JsValue::Object(crate::js::JsObject::new()),
        ];
        assert_eq!(array_join(&items, ","), "a,,1.5,true,1,,[object Object]");
    }
}
