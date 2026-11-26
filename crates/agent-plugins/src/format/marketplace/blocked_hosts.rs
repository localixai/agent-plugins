//! Port of `src/format/marketplace/blocked-hosts.ts`, switched.
//!
//! Hosts a download may not point at: "can't point at a loopback, link-local, or cloud-metadata
//! host" (plugins/marketplace-reference.md, "archive plugin source"; plugins/host-marketplace.md,
//! "Stay within the download limits for hosted files"). The docs name three categories, not the
//! hosts in them, so the check is lexical, over the address as written, and the lists are the
//! kit's judgment (spec/judgments.md). The kit resolves no name: a DNS name that merely resolves to
//! a blocked address is for the host's `HttpClient` egress policy.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{Pattern, parse_int_hex, regex_test, to_int32};

/// Names that are loopback or cloud-metadata endpoints by name.
const BLOCKED_NAMES: [&str; 2] = ["localhost", "metadata.google.internal"];
/// Suffixes of such names.
const BLOCKED_SUFFIXES: [&str; 1] = [".localhost"];

/// Cloud metadata services that are not in a link-local range (Alibaba Cloud).
const BLOCKED_IPV4: [&str; 1] = ["100.100.100.200"];

/// An IPv4 address as written: four groups of one to three ASCII digits.
static IPV4: Pattern =
    LazyLock::new(|| Regex::new(r"^([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})$").ok());

/// What JavaScript's `Number(match[n])` reads from one group of an [`IPV4`] match.
fn octet(captures: &regex::Captures<'_>, index: usize) -> u32 {
    captures
        .get(index)
        .and_then(|group| group.as_str().parse::<u32>().ok())
        .unwrap_or(0)
}

fn is_blocked_ipv4(host: &str) -> bool {
    if BLOCKED_IPV4.contains(&host) {
        return true;
    }
    let Some(captures) = IPV4.as_ref().and_then(|regex| regex.captures(host)) else {
        return false;
    };
    let first = octet(&captures, 1);
    let second = octet(&captures, 2);
    // 0.0.0.0/8 is "this host on this network" (RFC 1122): a connection to it can reach this machine.
    let loopback = first == 127 || first == 0;
    let link_local = first == 169 && second == 254;
    loopback || link_local
}

fn groups_of(text: &str) -> Vec<f64> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split(':').map(parse_int_hex).collect()
    }
}

/// Characters an IPv6 address as written may hold, in either letter case.
static IPV6_CHARACTERS: Pattern = LazyLock::new(|| Regex::new(r"^[0-9A-Fa-f:.]+$").ok());

/// The eight 16-bit groups of an IPv6 address as written (`::` expanded), or `None` when it is not
/// one.
fn ipv6_groups(host: &str) -> Option<Vec<f64>> {
    let sides: Vec<&str> = host.split("::").collect();
    if sides.len() > 2 || !regex_test(&IPV6_CHARACTERS, host) {
        return None;
    }
    let head = groups_of(sides.first().copied().unwrap_or(""));
    let tail = if sides.len() == 2 {
        groups_of(sides.get(1).copied().unwrap_or(""))
    } else {
        Vec::new()
    };
    let fill = 8 - i64::try_from(head.len()).unwrap_or(0) - i64::try_from(tail.len()).unwrap_or(0);
    let refused = if sides.len() == 1 {
        head.len() != 8
    } else {
        fill < 1
    };
    if refused {
        return None;
    }
    let zeros = if sides.len() == 2 {
        usize::try_from(fill).unwrap_or(0)
    } else {
        0
    };
    let groups: Vec<f64> = head
        .into_iter()
        .chain(std::iter::repeat_n(0.0, zeros))
        .chain(tail)
        .collect();
    if groups.iter().any(|group| group.is_nan()) {
        None
    } else {
        Some(groups)
    }
}

const AWS_IPV6_METADATA: [u16; 8] = [0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254];

fn is_blocked_ipv6(host: &str) -> bool {
    let Some(groups) = ipv6_groups(host) else {
        return false;
    };
    let first = groups.first().copied().unwrap_or(0.0);
    let zeros = groups.iter().take(7).all(|group| *group == 0.0);
    let last = groups.get(7).copied();
    let loopback = zeros && (last == Some(1.0) || last == Some(0.0));
    let link_local = (to_int32(first) & 0xffc0) == 0xfe80;
    let metadata = AWS_IPV6_METADATA
        .iter()
        .enumerate()
        .all(|(index, group)| groups.get(index) == Some(&f64::from(*group)));
    loopback || link_local || metadata || is_blocked_mapped(&groups)
}

/// `::ffff:a.b.c.d`, an IPv4 address in IPv6 clothes: the URL parser writes it as two hex groups.
fn is_blocked_mapped(groups: &[f64]) -> bool {
    let mapped =
        groups.iter().take(5).all(|group| *group == 0.0) && groups.get(5) == Some(&65_535.0);
    if !mapped {
        return false;
    }
    let high = to_int32(groups.get(6).copied().unwrap_or(0.0));
    let low = to_int32(groups.get(7).copied().unwrap_or(0.0));
    is_blocked_ipv4(&format!(
        "{}.{}.{}.{}",
        high >> 8,
        high & 0xff,
        low >> 8,
        low & 0xff
    ))
}

/// True when `hostname` (as `URL.hostname` writes it, IPv6 in brackets) is loopback, link-local or
/// a metadata host.
#[must_use]
pub fn is_blocked_host(hostname: &str) -> bool {
    let lowered = hostname.to_lowercase();
    // `.replace(/^\[|\]$/g, '')`: one opening bracket in front, one closing at the end.
    let unbracketed = lowered.strip_prefix('[').unwrap_or(&lowered);
    let unbracketed = unbracketed.strip_suffix(']').unwrap_or(unbracketed);
    // `.replace(/\.+$/, '')`
    let host = unbracketed.trim_end_matches('.');
    if BLOCKED_NAMES.contains(&host) {
        return true;
    }
    if BLOCKED_SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) {
        return true;
    }
    if host.contains(':') {
        is_blocked_ipv6(host)
    } else {
        is_blocked_ipv4(host)
    }
}

crate::registry::exports! {
    "format/marketplace/blocked-hosts" {
        "isBlockedHost" => ok |hostname: String| is_blocked_host(&hostname);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { isBlockedHost } from "./src/format/marketplace/blocked-hosts.ts";
    //   console.log(JSON.stringify(["[f54d0e2a10bfe4b::1]", "[5b09bc08a4ffeab::1]",
    //   "[3b96f9f274efeae::1]"].map(isBlockedHost)))'
    // prints [true,false,false]: `Number.parseInt` of a hexadecimal group longer than 53 bits is
    // not the correctly rounded number, and the `0xffc0` mask reads the difference.
    #[test]
    fn a_long_group_is_read_as_the_runtime_reads_it() {
        assert!(is_blocked_host("[f54d0e2a10bfe4b::1]"));
        assert!(!is_blocked_host("[5b09bc08a4ffeab::1]"));
        assert!(!is_blocked_host("[3b96f9f274efeae::1]"));
    }
}
