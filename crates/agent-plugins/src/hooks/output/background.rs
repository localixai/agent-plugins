//! Port of `src/hooks/output/background.ts`, switched.

use serde::Serialize;

use crate::hooks::verdict::types::BackgroundDelivery;
use crate::js::promise_all;
use crate::ports::BoxFuture;
use crate::registry::{HostPromise, catch_host};

/// What a host takes from the deliveries of background hooks (hooks.md, "How async hooks
/// execute"): the `additionalContext` and `systemMessage` texts "for Claude on the next
/// conversation turn", and the text an `asyncRewake` hook that exited 2 "wakes Claude" with,
/// surfaced apart so the host can open a turn for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettledDeliveries {
    /// The `additionalContext` texts, in order.
    pub additional_context: Vec<String>,
    /// The `systemMessage` texts, in order.
    pub system_messages: Vec<String>,
    /// Present when at least one hook woke Claude; several wake texts join with a newline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake: Option<String>,
}

/// The deliveries as one value, in the order given; pure.
#[must_use]
pub fn group_deliveries(deliveries: &[BackgroundDelivery]) -> SettledDeliveries {
    let wakes: Vec<&str> = deliveries
        .iter()
        .filter_map(|delivery| delivery.wake.as_deref())
        .collect();
    SettledDeliveries {
        additional_context: deliveries
            .iter()
            .filter_map(|delivery| delivery.additional_context.clone())
            .collect(),
        system_messages: deliveries
            .iter()
            .filter_map(|delivery| delivery.system_message.clone())
            .collect(),
        wake: (!wakes.is_empty()).then(|| wakes.join("\n")),
    }
}

/// Waits for the promises of `HookOutcome.background` and groups what they delivered, in the order
/// of the promises. Judgment (spec/judgments.md): a promise that rejects (the runner lost the
/// process) delivers nothing and does not hide the others; the dispatch already reported the run.
pub async fn settle_deliveries(
    background: Vec<HostPromise<BackgroundDelivery>>,
) -> SettledDeliveries {
    let settled = promise_all(
        background
            .into_iter()
            .map(|promise| -> BoxFuture<'static, Vec<BackgroundDelivery>> {
                Box::pin(async move {
                    catch_host(promise)
                        .await
                        .map_or_else(|_| vec![], |delivery| vec![delivery])
                })
            })
            .collect(),
    )
    .await;
    group_deliveries(&settled.into_iter().flatten().collect::<Vec<_>>())
}

crate::registry::exports! {
    "hooks/output/background" {
        "groupDeliveries" => ok |deliveries: Vec<BackgroundDelivery>| group_deliveries(&deliveries);
        "settleDeliveries" => async ok |background: Vec<HostPromise<BackgroundDelivery>>|
            settle_deliveries(background).await;
    }
}
