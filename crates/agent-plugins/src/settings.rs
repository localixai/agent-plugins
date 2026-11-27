//! Layer 4: the settings (`src/settings/` in the npm package): how the layers combine, and the
//! decision each area of them makes. A module taking the host's `PolicyView` or `Environment`
//! (objects with methods, read synchronously) is ported over the traits of [`view`] and
//! `ports::environment` and stays TypeScript in the package, whose boundary carries no such host
//! value to a synchronous call; the exports of such a module are only its data-in, data-out names.

pub mod broken_file;
pub mod canonical;
pub mod channels;
pub mod customization;
pub mod general;
pub mod hooks;
pub mod host_switches;
pub mod layers;
pub mod managed;
pub mod marketplaces;
pub mod mcp;
pub mod paths;
pub mod permissions;
pub mod plugins;
pub mod repair;
pub mod skills;
pub mod view;
pub mod workflows;

use serde::{Serialize, Serializer};

use crate::js::JsValue;

/// A member the TypeScript types `T | undefined` and always writes (`{ by: undefined }`): `None`
/// is written as `undefined`, as the wire tags it, never as `null` and never left out.
///
/// # Errors
///
/// What the serializer refuses.
pub fn undefined_if_none<T: Serialize, S: Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => value.serialize(serializer),
        None => JsValue::Undefined.serialize(serializer),
    }
}
