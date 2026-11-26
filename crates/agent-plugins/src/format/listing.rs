//! Port of `src/format/listing.ts`, switched.
//! The one order of names the crate lists by: UTF-16 code units, so an order never depends on the
//! host's locale or its port. Listing a folder sorted is the loading stream's, over its port.

use std::cmp::Ordering;

use crate::js::compare_code_units;

/// The order of two names by UTF-16 code units.
#[must_use]
pub fn by_code_unit(first: &str, second: &str) -> Ordering {
    compare_code_units(first, second)
}

crate::registry::exports! {
    "format/listing" {
        "byCodeUnit" => ok |first: String, second: String| crate::registry::sign(by_code_unit(&first, &second));
    }
}
