//! Port of `src/updates/auto-update-pass.ts`, switched.
//!
//! The auto-update pass (plugins/loading.md, "When auto-update runs"): "In an interactive session,
//! after you send your first message, Claude Code waits a random delay of up to ten minutes. It then
//! refreshes every marketplace with auto-update on and updates the plugins installed from them on
//! disk." The kit plans one pass and arms the first one on the `Clock`; how often a host repeats it
//! is the host's choice (the docs describe the one pass of a session).

use serde::{Deserialize, Serialize};

use super::auto_update_precedence::auto_update_pass_allowed;
use super::auto_update_skips::{
    AutoUpdateCandidate, AutoUpdateSkip, SkippedBy, plugin_skipped_by_auto_update,
};
use crate::js::JsValue;
use crate::js::{math_max, math_min};
use crate::ports::environment::Environment;
use crate::registry::{Callback, Function, Port};

/// Ten minutes.
pub const FIRST_PASS_MAX_DELAY_MS: f64 = 10.0 * 60_000.0;

/// A delay in `[0, 10 minutes]` from a number in `[0, 1]` (anything outside is clamped).
#[must_use]
pub fn first_pass_delay_of(random: f64) -> f64 {
    math_min(
        FIRST_PASS_MAX_DELAY_MS,
        math_max(0.0, (random * (FIRST_PASS_MAX_DELAY_MS + 1.0)).floor()),
    )
}

/// `firstPassDelayMs`: a delay in `[0, 10 minutes]` from a source of numbers in `[0, 1]`.
pub async fn first_pass_delay_ms(random: Callback) -> f64 {
    first_pass_delay_of(random.call::<f64>(vec![]).await)
}

/// The cancel of a pass that was not armed: it does nothing.
struct Nothing;

crate::registry::methods! {
    Nothing {
        "call" => ok |_this| ();
    }
}

/// How the pass is cancelled: the clock's own cancel, or nothing.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Cancel {
    /// The host's clock's cancel.
    Host(Callback),
    /// Nothing was set on the clock.
    Nothing(Function),
}

/// What `armFirstPass` answers.
#[derive(Debug, Serialize)]
pub struct ArmedPass {
    /// False when the environment turns the pass off: nothing was set on the clock.
    pub armed: bool,
    /// Cancels the pass.
    pub cancel: Cancel,
}

/// Call this after the first message. The pass is armed only when `autoUpdatePassAllowed`.
/// `options` holds the host's functions, so it crosses as a port and each member is read from it;
/// `run` is handed to the clock as the host's own function.
pub async fn arm_first_pass(options: Port) -> ArmedPass {
    let env: Port = options.member("env").await;
    if !auto_update_pass_allowed(&env).await {
        return ArmedPass {
            armed: false,
            cancel: Cancel::Nothing(Function::new(Nothing)),
        };
    }
    let clock: Port = options.member("clock").await;
    let run: JsValue = options.member("run").await;
    let random: Callback = options.member("random").await;
    let delay = first_pass_delay_ms(random).await;
    let cancel = clock
        .method::<Callback>("setTimer", vec![run, JsValue::from(delay)])
        .await;
    ArmedPass {
        armed: true,
        cancel: Cancel::Host(cancel),
    }
}

/// One plugin the user installed from a marketplace.
#[derive(Clone, Debug, Deserialize)]
pub struct PassPlugin {
    /// Its id.
    pub id: String,
    /// What the skip rules read.
    pub candidate: AutoUpdateCandidate,
}

/// A marketplace the pass may refresh.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassMarketplace {
    /// Its name.
    pub name: String,
    /// Its resolved auto-update setting (`marketplaceAutoUpdate`).
    pub auto_update: bool,
    /// The plugins the user installed from it.
    pub plugins: Vec<PassPlugin>,
}

/// A plugin the pass leaves alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassSkipped {
    /// Its id.
    pub id: String,
    /// Why.
    pub reason: SkippedBy,
    /// The user is told in the Errors tab to install or update it by hand.
    pub listed_in_errors_tab: bool,
}

/// What one pass does.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PassPlan {
    /// Marketplaces to refresh, in the order given.
    pub refresh: Vec<String>,
    /// Plugin ids to update on disk.
    pub update: Vec<String>,
    /// The plugins skipped.
    pub skipped: Vec<PassSkipped>,
}

/// One pass: nothing when the environment turns it off, otherwise per auto-update marketplace its plugins less the skipped.
pub async fn plan_auto_update_pass(
    marketplaces: &[PassMarketplace],
    env: &dyn Environment,
) -> PassPlan {
    if !auto_update_pass_allowed(env).await {
        return PassPlan::default();
    }
    let on: Vec<&PassMarketplace> = marketplaces.iter().filter(|one| one.auto_update).collect();
    let verdicts: Vec<(&str, AutoUpdateSkip)> = on
        .iter()
        .flat_map(|marketplace| &marketplace.plugins)
        .map(|plugin| {
            (
                plugin.id.as_str(),
                plugin_skipped_by_auto_update(&plugin.candidate),
            )
        })
        .collect();
    PassPlan {
        refresh: on
            .iter()
            .map(|marketplace| marketplace.name.clone())
            .collect(),
        update: verdicts
            .iter()
            .filter(|(_, skip)| matches!(skip, AutoUpdateSkip::Kept { .. }))
            .map(|(id, _)| (*id).to_owned())
            .collect(),
        skipped: verdicts
            .iter()
            .filter_map(|(id, skip)| match skip {
                AutoUpdateSkip::Kept { .. } => None,
                AutoUpdateSkip::Skipped {
                    reason,
                    listed_in_errors_tab,
                    ..
                } => Some(PassSkipped {
                    id: (*id).to_owned(),
                    reason: *reason,
                    listed_in_errors_tab: *listed_in_errors_tab,
                }),
            })
            .collect(),
    }
}

/// What the running session shows after a plugin updated on disk: it keeps the version it loaded.
#[must_use]
pub fn plugin_updated_notice(name: &str) -> String {
    format!("Plugin updated: {name} · Run /reload-plugins to apply")
}

crate::registry::exports! {
    "updates/auto-update-pass" {
        "FIRST_PASS_MAX_DELAY_MS" => ok || FIRST_PASS_MAX_DELAY_MS;
        "firstPassDelayMs" => async ok |random: Callback| first_pass_delay_ms(random).await;
        "armFirstPass" => async ok |options: Port| arm_first_pass(options).await;
        "planAutoUpdatePass" => async ok |marketplaces: Vec<PassMarketplace>, env: Port|
            plan_auto_update_pass(&marketplaces, &env).await;
        "pluginUpdatedNotice" => ok |name: String| plugin_updated_notice(&name);
    }
}

#[cfg(test)]
mod tests {
    use super::first_pass_delay_of;

    #[test]
    fn a_delay_is_clamped_and_keeps_nan() {
        assert_eq!(first_pass_delay_of(0.5), 300_000.0);
        assert_eq!(first_pass_delay_of(2.0), 600_000.0);
        assert_eq!(first_pass_delay_of(-1.0), 0.0);
        assert!(first_pass_delay_of(f64::NAN).is_nan());
    }
}
