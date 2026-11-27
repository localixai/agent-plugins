//! Port of `src/settings/managed/load.ts`, switched.
//!
//! What a host found at each managed source of the machine (managed-settings.md, "Choose a delivery
//! mechanism"): the kit reads no file, plist, registry value or fetch. A source the machine does not
//! have is left out. `labels` name a source in a report: a path, `per-user managed preferences`,
//! `Registry: HKLM\SOFTWARE\Policies\ClaudeCode\Settings`.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::settings::managed::admin_source::{
    AdminDocument, ManagedSource, SourceRead, read_admin_document,
};
use crate::settings::managed::compose::{ComposeOptions, ManagedComposition, compose_managed};
use crate::settings::managed::drop_in::{
    FileLabel, FileLabelNames, ManagedFileRead, read_file_document,
};

/// What the host found at each source it has.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedReads {
    /// Server-managed settings.
    #[serde(default)]
    pub remote: Option<SourceRead>,
    /// The MDM profile.
    #[serde(default)]
    pub plist: Option<SourceRead>,
    /// The `HKLM` registry.
    #[serde(default)]
    pub hklm: Option<SourceRead>,
    /// `managed-settings.json` and the `managed-settings.d/` folder beside it.
    #[serde(default)]
    pub file: Option<ManagedFileRead>,
    /// The `HKCU` registry.
    #[serde(default)]
    pub hkcu: Option<SourceRead>,
    /// What names each source in a report.
    #[serde(default)]
    pub labels: Option<IndexMap<ManagedSource, String>>,
}

/// The documents of the sources the host found, in no particular order.
async fn read_managed_documents(reads: &ManagedReads) -> Vec<AdminDocument> {
    let label = |source: ManagedSource| {
        reads
            .labels
            .as_ref()
            .and_then(|labels| labels.get(&source))
            .map(String::as_str)
    };
    let single = |source: ManagedSource, read: Option<&SourceRead>| {
        read.map(|read| read_admin_document(source, Some(read), label(source)))
    };
    let file = match &reads.file {
        None => None,
        Some(file) => {
            let mut file = file.clone();
            if file.label.is_none()
                && let Some(base) = label(ManagedSource::File)
            {
                file.label = Some(FileLabel::Data(FileLabelNames {
                    base: Some(base.to_owned()),
                    directory: None,
                }));
            }
            Some(read_file_document(&file).await)
        }
    };
    [
        single(ManagedSource::Remote, reads.remote.as_ref()),
        single(ManagedSource::Plist, reads.plist.as_ref()),
        single(ManagedSource::Hklm, reads.hklm.as_ref()),
        file,
        single(ManagedSource::Hkcu, reads.hkcu.as_ref()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The managed tier of the sources the host found.
pub async fn compose_managed_reads(
    reads: &ManagedReads,
    options: ComposeOptions,
) -> ManagedComposition {
    compose_managed(read_managed_documents(reads).await, options)
}

crate::registry::exports! {
    "settings/managed/load" {
        "composeManagedReads" => async ok |reads: ManagedReads, options: Option<ComposeOptions>|
            compose_managed_reads(&reads, options.unwrap_or_default()).await;
    }
}
