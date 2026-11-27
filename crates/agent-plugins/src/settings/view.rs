//! The core's counterpart of the `PolicyView` interface of `src/settings/view.ts`: typed
//! questions over the ordered layers, which every area resolver asks. The package builds it
//! (`createPolicyView`) from the key rows' closures, so it stays TypeScript and crosses as a host
//! value: its methods answer at once there, and here they return futures, so a host across the
//! boundary answers each as a need (`crate::registry`), and a Rust host returns ready ones.

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::Port;
use crate::settings::layers::{KeyHolder, PolicyLayer};

/// Typed questions over the ordered layers (docs/API.md, "Settings"). A key is the name the
/// package uses (`allowedChannelPlugins`, `permissions.defaultMode`).
pub trait PolicyView: Send + Sync {
    /// The layers, highest precedence first.
    fn layers(&self) -> BoxFuture<'_, Vec<PolicyLayer>>;
    /// The key as the files combine it (`None` when no file sets it).
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<JsValue>>;
    /// The highest layer that sets the key.
    fn winner<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<KeyHolder>>;
    /// Every layer that sets the key, highest first: what a key with its own rule walks.
    fn each_from_highest<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Vec<KeyHolder>>;
}

/// The host's view: `layers` read as its member, each question as its method.
impl PolicyView for Port {
    fn layers(&self) -> BoxFuture<'_, Vec<PolicyLayer>> {
        Box::pin(self.member::<Vec<PolicyLayer>>("layers"))
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<JsValue>> {
        let asked = self.method::<JsValue>("get", vec![key.into()]);
        Box::pin(async move { Some(asked.await).filter(|value| !value.is_undefined()) })
    }

    fn winner<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<KeyHolder>> {
        Box::pin(self.method::<Option<KeyHolder>>("winner", vec![key.into()]))
    }

    fn each_from_highest<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Vec<KeyHolder>> {
        Box::pin(self.method::<Vec<KeyHolder>>("eachFromHighest", vec![key.into()]))
    }
}
