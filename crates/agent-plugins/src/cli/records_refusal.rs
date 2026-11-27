//! Ported from `src/cli/records-refusal.ts`; only the core uses it now, so it has no wrapper.
//!
//! While `installed_plugins.json` holds a record this version cannot read, or is in a format it
//! does not know, `claude plugin install`, `uninstall` and `update` refuse before anything else
//! (§16.6.2 item 1): `Plugin "<name>" was not installed:` (`uninstalled`, `updated`) and the text
//! `plugin list` prints as its note, with `failureCode: "install_records_unreadable"`.
//! `unreadable` is that text, as `readInstalledPlugins` reported it; `None` when every record was
//! read.

use super::result::{CommandOutput, Extras, command_failed};
use crate::marketplace::install_messages::{RefusingCommand, refuse_for_unreadable_records};

/// The refusal, or `None` when every record was read.
#[must_use]
pub fn records_refusal(
    command: RefusingCommand,
    name: &str,
    unreadable: Option<&str>,
) -> Option<CommandOutput> {
    let unreadable = unreadable?;
    let refusal = refuse_for_unreadable_records(command, name, unreadable);
    Some(command_failed(
        command.as_str(),
        &refusal.message,
        Extras {
            failure_code: Some(refusal.failure_code),
            fields: None,
        },
    ))
}
