//! Port of `src/marketplace/header-map.ts`, switched.
//!
//! Header maps as the kit passes them to a download: names lower case, as on the `HttpClient` port
//! (`HeaderMap`). HTTP header names are case-insensitive, so "a header of the same name"
//! (plugins/host-marketplace.md, "Authenticate archive downloads") is compared without regard to
//! case; of two spellings in one map the later wins (spec/judgments.md).

use indexmap::IndexMap;

use super::types::HeaderMap;
use crate::format::marketplace::header_names::is_routing_header;
use crate::js::in_key_order;

/// The names of `headers` lower-cased, as a plain object holds them.
fn lowered(headers: &IndexMap<String, String>) -> HeaderMap {
    let mut lowered = HeaderMap::new();
    for (name, value) in headers {
        lowered.insert(name.to_lowercase(), value.clone());
    }
    lowered
}

/// Header maps as the kit passes them to a download: names lower case, as on the `HttpClient` port.
#[must_use]
pub fn normalize_headers(headers: &IndexMap<String, String>) -> HeaderMap {
    in_key_order(lowered(headers))
}

/// `later` replaces a header of the same name in `earlier`: the entry's value over the source's.
#[must_use]
pub fn merge_headers(earlier: &HeaderMap, later: &HeaderMap) -> HeaderMap {
    let mut merged = lowered(earlier);
    for (name, value) in lowered(later) {
        merged.insert(name, value);
    }
    in_key_order(merged)
}

/// `headers` without the request-routing and client-identity names; authentication names stay.
#[must_use]
pub fn without_routing_headers(headers: &HeaderMap) -> HeaderMap {
    headers
        .iter()
        .filter(|(name, _)| !is_routing_header(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

crate::registry::exports! {
    "marketplace/header-map" {
        "normalizeHeaders" => ok |headers: IndexMap<String, String>| normalize_headers(&headers);
        "mergeHeaders" => ok |earlier: HeaderMap, later: HeaderMap| merge_headers(&earlier, &later);
        "withoutRoutingHeaders" => ok |headers: HeaderMap| without_routing_headers(&headers);
    }
}
