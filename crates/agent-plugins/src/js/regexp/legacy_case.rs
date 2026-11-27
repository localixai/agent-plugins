//! ECMAScript's `Canonicalize` for a pattern without `u` or `v`: a code unit is its `toUpperCase`
//! when that is one code unit, except that a unit past ASCII never becomes an ASCII one (`ſ` stays
//! `ſ`, `ı` stays `ı`). Two units match each other under `i` when they canonicalize alike. The
//! engine folds such a pattern by Unicode's case folding instead, which relates `s` and `ſ`, `k`
//! and the Kelvin sign, `ß` and `ẞ`, `ω` and the Ohm sign; so the core spells each cased unit of
//! such a pattern as the class of the units it matches and compiles it without `i`
//! (`source.rs`).

use std::sync::LazyLock;

/// Every BMP code unit's canonical unit, and the units sorted by it.
struct Table {
    canonical: Vec<u16>,
    by_canonical: Vec<(u16, u16)>,
}

/// `Canonicalize(rer, ch)` with `rer.[[IgnoreCase]]` and not `rer.[[Unicode]]`.
fn canonicalize(unit: u16) -> u16 {
    let Some(character) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mut upper = character.to_uppercase();
    let (Some(first), None) = (upper.next(), upper.next()) else {
        return unit;
    };
    let Ok(single) = u16::try_from(u32::from(first)) else {
        return unit;
    };
    if unit >= 128 && single < 128 {
        unit
    } else {
        single
    }
}

static TABLE: LazyLock<Table> = LazyLock::new(|| {
    let canonical: Vec<u16> = (0..=u16::MAX).map(canonicalize).collect();
    let mut by_canonical: Vec<(u16, u16)> = canonical
        .iter()
        .zip(0..=u16::MAX)
        .map(|(canon, unit)| (*canon, unit))
        .collect();
    by_canonical.sort_unstable();
    Table {
        canonical,
        by_canonical,
    }
});

/// `unit` canonicalized.
pub(super) fn canonical(unit: u16) -> u16 {
    TABLE
        .canonical
        .get(usize::from(unit))
        .copied()
        .unwrap_or(unit)
}

/// The units that canonicalize as `unit` does (`unit` among them).
pub(super) fn partners(unit: u16) -> &'static [(u16, u16)] {
    let table = &*TABLE;
    let canon = canonical(unit);
    let start = table
        .by_canonical
        .partition_point(|(each, _)| *each < canon);
    let end = table
        .by_canonical
        .partition_point(|(each, _)| *each <= canon);
    table.by_canonical.get(start..end).unwrap_or_default()
}

/// Whether a unit matches any unit but itself under `i`.
pub(super) fn is_cased(unit: u32) -> bool {
    u16::try_from(unit).is_ok_and(|unit| partners(unit).len() > 1)
}

/// The units a class of `ranges` matches under `i`, as sorted, merged ranges.
pub(super) fn closure(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut units: Vec<u32> = Vec::new();
    for (low, high) in ranges {
        for unit in *low..=*high {
            units.push(unit);
            if let Ok(unit) = u16::try_from(unit) {
                units.extend(partners(unit).iter().map(|(_, each)| u32::from(*each)));
            }
        }
    }
    units.sort_unstable();
    units.dedup();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for unit in units {
        match merged.last_mut() {
            Some((_, high)) if *high + 1 == unit => *high = unit,
            _ => merged.push((unit, unit)),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::{closure, is_cased};

    #[test]
    fn keeps_units_past_ascii_from_folding_into_ascii() {
        assert_eq!(closure(&[(0x73, 0x73)]), [(0x53, 0x53), (0x73, 0x73)]);
        assert!(!is_cased(0x17F));
        assert!(!is_cased(0x131));
        assert!(!is_cased(0xDF));
        assert!(is_cased(0x3C3));
        assert_eq!(closure(&[(0x3C2, 0x3C2)]), [(0x3A3, 0x3A3), (0x3C2, 0x3C3)]);
    }
}
