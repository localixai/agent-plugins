//! Port of `src/settings/customization/surfaces.ts`, switched.
//!
//! The kinds of customization `strictPluginOnlyCustomization` locks.

use serde::Serialize;

use crate::js::JsValue;
use crate::protocol::settings_keys::PLUGIN_ONLY_SURFACES;
use crate::registry::Set;

/// The kinds of customization the lock covers, and how many names in the list were no kind.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceLocks {
    /// The kinds locked.
    pub locked: Set<&'static str>,
    /// Names this version does not recognize: they lock nothing, and a status note counts them.
    pub unknown_names: f64,
}

/// `ALL`: every kind.
fn all() -> Set<&'static str> {
    Set {
        members: PLUGIN_ONLY_SURFACES.to_vec(),
    }
}

fn locks(locked: Set<&'static str>, unknown_names: usize) -> SurfaceLocks {
    SurfaceLocks {
        locked,
        unknown_names: f64::from(u32::try_from(unknown_names).unwrap_or(u32::MAX)),
    }
}

fn surface_named(name: &JsValue) -> Option<&'static str> {
    PLUGIN_ONLY_SURFACES
        .iter()
        .copied()
        .find(|surface| name.as_str() == Some(*surface))
}

/// `new Set(known)`: the kinds named, each once, in the order first named.
fn set_of(known: &[&'static str]) -> Set<&'static str> {
    let mut members: Vec<&'static str> = Vec::new();
    for surface in known {
        if !members.contains(surface) {
            members.push(surface);
        }
    }
    Set { members }
}

fn from_list(names: &[JsValue]) -> SurfaceLocks {
    let known: Vec<&'static str> = names.iter().filter_map(surface_named).collect();
    locks(set_of(&known), names.len() - known.len())
}

/// What the value of `strictPluginOnlyCustomization` locks (settings-reference.md,
/// `strictPluginOnlyCustomization`): `true` locks all four kinds, an array locks the kinds it
/// names, and anything unset locks nothing. A name no kind has "locks nothing; a status note counts
/// such entries" (managed-settings.md, "Invalid entries in managed settings"), the same page that
/// treats a value "neither a boolean nor an array" as `true`. `false` is the SDK type's "explicit
/// no-op" (spec/judgments.md).
#[must_use]
pub fn read_surface_locks(value: &JsValue) -> SurfaceLocks {
    match value {
        JsValue::Undefined | JsValue::Null | JsValue::Bool(false) => locks(
            Set {
                members: Vec::new(),
            },
            0,
        ),
        JsValue::Array(names) => from_list(names),
        _ => locks(all(), 0),
    }
}

crate::registry::exports! {
    "settings/customization/surfaces" {
        "readSurfaceLocks" => ok |value: JsValue| read_surface_locks(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_locks_the_kinds_it_names_once() {
        let names = vec![
            JsValue::String("mcp".to_owned()),
            JsValue::String("mcp".to_owned()),
            JsValue::Number(1.0),
            JsValue::String("skills".to_owned()),
        ];
        let read = read_surface_locks(&JsValue::Array(names));
        assert_eq!(read.locked.members, ["mcp", "skills"]);
        assert_eq!(read.unknown_names, 1.0);
    }
}
