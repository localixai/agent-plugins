//! Port of `src/loading/core/dual-check.ts`, switched.
//!
//! The dual-package check a host's validator runs (Agent Plugins Extended §18.3.1, §18.1.5 item 6):
//! on a root with both manifests, what of the shared manifest fields loads in one form only. It adds
//! to `validatePlugin`, which validates the Claude Code view alone, as `claude plugin validate` does
//! (J1.1). A root that is not a dual package gives `dual: false` and no findings.

use serde::{Deserialize, Serialize};

use super::form::has_core_manifest;
use crate::format::core::dual::{
    CLAUDE_MANIFEST_FILE, CORE_MANIFEST_FILE, DualIssue, check_dual_manifests,
};
use crate::format::read_json::{JsonRead, read_json};
use crate::format::validate::finding::error_in;
use crate::js::{JsValue, promise_all_pair};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// What `checkDualPackage` is given.
#[derive(Debug, Deserialize)]
pub struct DualInput {
    /// The plugin root.
    pub files: Port,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// Whether the root is a dual package, and what loads in one form only.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DualCheck {
    /// Both manifests are there.
    pub dual: bool,
    /// What loads in one form only.
    pub issues: Vec<DualIssue>,
}

/// The dual-package check of a root.
pub async fn check_dual_package(files: &dyn PluginFiles, signal: Option<&Signal>) -> DualCheck {
    let (claude_present, core) = promise_all_pair(
        files.exists(CLAUDE_MANIFEST_FILE, signal),
        Box::pin(has_core_manifest(files, signal)),
    )
    .await;
    if !claude_present || !core {
        return DualCheck {
            dual: false,
            issues: Vec::new(),
        };
    }
    let (core_read, claude_read) = promise_all_pair(
        read_json(files, CORE_MANIFEST_FILE, signal),
        read_json(files, CLAUDE_MANIFEST_FILE, signal),
    )
    .await;
    let claude = match claude_read {
        JsonRead::Value { value } => value,
        JsonRead::Problem { problem, .. } => {
            return DualCheck {
                dual: true,
                issues: vec![DualIssue::Found(error_in(
                    CLAUDE_MANIFEST_FILE,
                    "",
                    &format!("{CLAUDE_MANIFEST_FILE} is not valid JSON: {problem}"),
                ))],
            };
        }
    };
    let core_value = match core_read {
        JsonRead::Value { value } => value,
        JsonRead::Problem { .. } => JsValue::Undefined,
    };
    DualCheck {
        dual: true,
        issues: check_dual_manifests(&core_value, &claude),
    }
}

crate::registry::exports! {
    "loading/core/dual-check" {
        "checkDualPackage" => async ok |input: DualInput|
            check_dual_package(&input.files, input.signal.as_ref()).await;
    }
}
