//! Ported from `src/settings/repair/network.ts`; only the core uses it now, so it has no wrapper.
//!
//! `gatewayInternalNetworks` (settings-reference.md, `gatewayInternalNetworks`; claude-apps-gateway.md,
//! "Allow a gateway on public address space you own"): an array of strings, at most four IPv4 CIDR
//! blocks, each `/8` to `/32`, not overlapping one another, none overlapping private space or space
//! that is never an organization's network. `240.0.0.0/4` is allowed.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{JsValue, Pattern};

const MAX_BLOCKS: usize = 4;
const MIN_PREFIX: u32 = 8;

/// The addresses of a block, as numbers.
#[derive(Clone, Copy)]
struct Range {
    first: u64,
    last: u64,
}

static CIDR: Pattern = LazyLock::new(|| {
    Regex::new(r"^([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})/([0-9]{1,2})$").ok()
});

/// The addresses a block of prefix `minPrefix` to `/32` covers, or `None` for text that is no such IPv4 CIDR block.
fn range_of(text: &str, min_prefix: u32) -> Option<Range> {
    let captures = CIDR.as_ref()?.captures(text)?;
    let number = |at: usize| -> Option<u64> { captures.get(at)?.as_str().parse().ok() };
    let octets = [number(1)?, number(2)?, number(3)?, number(4)?];
    let prefix = u32::try_from(number(5)?).ok()?;
    if octets.iter().any(|octet| *octet > 255) || prefix < min_prefix || prefix > 32 {
        return None;
    }
    let address = octets.iter().fold(0, |total, octet| total * 256 + octet);
    let size = 1_u64 << (32 - prefix);
    let first = address / size * size;
    Some(Range {
        first,
        last: first + size - 1,
    })
}

const fn overlap(a: Range, b: Range) -> bool {
    a.first <= b.last && b.first <= a.last
}

/// The ranges of `blocks`; a constant below is always a valid CIDR block.
fn ranges_of(blocks: &[&str]) -> Vec<Range> {
    blocks
        .iter()
        .filter_map(|block| range_of(block, 0))
        .collect()
}

/// Private address space, which `/login` already accepts without the key.
fn private_space() -> Vec<Range> {
    ranges_of(&[
        "10.0.0.0/8",
        "172.16.0.0/12",
        "192.168.0.0/16",
        "127.0.0.0/8",
        "169.254.0.0/16",
        "100.64.0.0/10",
    ])
}

/// Space that is never an organization's network: VPN and NAT64 local addresses, documentation ranges, reserved ranges and multicast.
fn never_a_network() -> Vec<Range> {
    ranges_of(&[
        "198.18.0.0/15",
        "192.0.0.0/24",
        "192.0.2.0/24",
        "198.51.100.0/24",
        "203.0.113.0/24",
        "0.0.0.0/8",
        "192.88.99.0/24",
        "224.0.0.0/4",
    ])
}

/// Why the value cannot be enforced, or `None` when it is a valid list of blocks.
#[must_use]
pub fn gateway_networks_fault(value: &JsValue) -> Option<String> {
    let Some(entries) = value.as_array() else {
        return Some("it must be a list of IPv4 CIDR blocks".to_owned());
    };
    if entries.len() > MAX_BLOCKS {
        return Some(format!("it lists more than {MAX_BLOCKS} blocks"));
    }
    let ranges: Vec<Option<Range>> = entries
        .iter()
        .map(|entry| entry.as_str().and_then(|text| range_of(text, MIN_PREFIX)))
        .collect();
    let blocks: Vec<Range> = ranges.iter().flatten().copied().collect();
    if blocks.len() < ranges.len() {
        return Some("a block is no IPv4 CIDR block of /8 to /32".to_owned());
    }
    let inside = |spaces: &[Range]| {
        blocks
            .iter()
            .any(|block| spaces.iter().any(|space| overlap(*block, *space)))
    };
    if inside(&private_space()) {
        return Some("a block overlaps private space".to_owned());
    }
    if inside(&never_a_network()) {
        return Some("a block overlaps space that is never an organization's network".to_owned());
    }
    let crossing = blocks.iter().enumerate().any(|(index, block)| {
        blocks
            .iter()
            .skip(index + 1)
            .any(|other| overlap(*block, *other))
    });
    crossing.then(|| "two blocks overlap".to_owned())
}
