//! The core's counterpart of the `PolicyView` interface of `src/settings/view.ts`: typed
//! questions over the ordered layers, which every area resolver asks. The package builds it
//! (`createPolicyView`) from the key rows' closures, so it stays TypeScript; a Rust host
//! implements this trait and the ported resolvers take it.

use crate::js::JsValue;
use crate::settings::layers::{PolicyLayer, PolicyScope};

/// One layer's value of a key, with the layer it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyHolder {
    /// The layer's scope.
    pub scope: PolicyScope,
    /// Position among the ordered layers: two files of one scope have two positions.
    pub position: usize,
    /// The key's value in that layer.
    pub value: JsValue,
}

/// Typed questions over the ordered layers (docs/API.md, "Settings"). A key is the name the
/// package uses (`allowedChannelPlugins`, `permissions.defaultMode`).
pub trait PolicyView {
    /// The layers, highest precedence first.
    fn layers(&self) -> &[PolicyLayer];
    /// The key as the files combine it (`None` when no file sets it).
    fn get(&self, key: &str) -> Option<JsValue>;
    /// The highest layer that sets the key.
    fn winner(&self, key: &str) -> Option<KeyHolder>;
    /// Every layer that sets the key, highest first: what a key with its own rule walks.
    fn each_from_highest(&self, key: &str) -> Vec<KeyHolder>;
}
