//! Port of `src/cli/marketplace-list.ts`, switched.
//!
//! `claude plugin marketplace list` (plugins/cli-reference.md, "plugin marketplace list"): the text
//! listing and the `--json` array, from the marketplaces the host reads out of its registry. Pure.
//! The layout of a text entry is the kit's (spec/judgments.md); the JSON fields are the docs'.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::result::Rendered;
use crate::js::{JsValue, json_stringify};
use crate::registry::Function;

/// The sources the docs' `source` field names: `github`, `git`, `url`, `directory`, `file` and
/// `claudeai`. A marketplace hosted on claude.ai has no local clone.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ListedSource {
    /// A GitHub repository.
    #[serde(rename_all = "camelCase")]
    Github {
        /// `owner/name`.
        repo: String,
        /// A branch or tag.
        #[serde(default, rename = "ref")]
        reference: Option<String>,
    },
    /// A git URL.
    #[serde(rename_all = "camelCase")]
    Git {
        /// The URL.
        url: String,
        /// A branch or tag.
        #[serde(default, rename = "ref")]
        reference: Option<String>,
    },
    /// A file at a URL.
    #[serde(rename_all = "camelCase")]
    Url {
        /// The URL.
        url: String,
    },
    /// A folder.
    #[serde(rename_all = "camelCase")]
    Directory {
        /// Its path.
        path: String,
    },
    /// A file.
    #[serde(rename_all = "camelCase")]
    File {
        /// Its path.
        path: String,
    },
    /// A marketplace hosted on claude.ai.
    #[serde(rename_all = "camelCase")]
    Claudeai {
        /// Its id.
        marketplace_id: String,
        /// The organization's UUID.
        organization_uuid: String,
    },
}

/// A marketplace the host's registry holds.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredMarketplace {
    /// Its name.
    pub name: String,
    /// Where it comes from.
    pub source: ListedSource,
    /// Where Claude Code cached it; a claude.ai marketplace has none.
    #[serde(default)]
    pub install_location: Option<String>,
    /// A claude.ai entry's recorded scope.
    #[serde(default)]
    pub scope: Option<String>,
    /// A claude.ai entry's status.
    #[serde(default)]
    pub status: Option<String>,
}

/// A marketplace claude.ai lists for the account that is not added yet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAiListing {
    /// Its name.
    pub name: String,
}

/// What the registry holds.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    /// The configured marketplaces.
    pub marketplaces: Vec<ConfiguredMarketplace>,
    /// What claude.ai lists.
    #[serde(default)]
    pub from_claude_ai: Option<Vec<ClaudeAiListing>>,
}

/// Every field is a string.
pub type ListedJson = IndexMap<String, String>;

/// The listing's `print(json)`: what the command prints.
#[derive(Debug)]
struct Print {
    json: Vec<ListedJson>,
    text: Vec<String>,
}

crate::registry::methods! {
    Print {
        "call" => ok |this, as_json: bool| this.print(as_json);
    }
}

impl Print {
    /// What the command prints: the array as the one line with `--json`, else the text; it exits 0.
    fn print(&self, as_json: bool) -> Rendered {
        let stdout = if as_json {
            vec![json_stringify(&listed_array(&self.json)).unwrap_or_default()]
        } else {
            self.text.clone()
        };
        Rendered {
            stdout,
            stderr: Vec::new(),
            exit_code: 0,
        }
    }
}

fn listed_array(json: &[ListedJson]) -> JsValue {
    JsValue::Array(
        json.iter()
            .map(|one| {
                JsValue::Object(
                    one.iter()
                        .map(|(key, value)| (key.clone(), JsValue::String(value.clone())))
                        .collect(),
                )
            })
            .collect(),
    )
}

/// The listing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPlan {
    /// The `--json` array: configured marketplaces only.
    pub json: Vec<ListedJson>,
    /// The text lines.
    pub text: Vec<String>,
    /// What the command prints: the array as the one line with `--json`, else the text; it exits 0.
    pub print: Function,
}

fn pinned(reference: Option<&String>) -> ListedJson {
    let mut fields = ListedJson::new();
    if let Some(reference) = reference {
        fields.insert("ref".to_owned(), reference.clone());
    }
    fields
}

fn with_ref(text: &str, reference: Option<&String>) -> String {
    reference.map_or_else(
        || text.to_owned(),
        |reference| format!("{text}#{reference}"),
    )
}

fn fields(pairs: &[(&str, &str)]) -> ListedJson {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// The JSON fields a source carries.
fn fields_of(source: &ListedSource) -> ListedJson {
    match source {
        ListedSource::Github { repo, reference } => {
            let mut json = fields(&[("source", "github"), ("repo", repo)]);
            json.extend(pinned(reference.as_ref()));
            json
        }
        ListedSource::Git { url, reference } => {
            let mut json = fields(&[("source", "git"), ("url", url)]);
            json.extend(pinned(reference.as_ref()));
            json
        }
        ListedSource::Url { url } => fields(&[("source", "url"), ("url", url)]),
        ListedSource::Directory { path } => fields(&[("source", "directory"), ("path", path)]),
        ListedSource::File { path } => fields(&[("source", "file"), ("path", path)]),
        ListedSource::Claudeai {
            marketplace_id,
            organization_uuid,
        } => fields(&[
            ("source", "claudeai"),
            ("marketplaceId", marketplace_id),
            ("organizationUuid", organization_uuid),
        ]),
    }
}

/// The text after `Source:`.
fn describe_source(source: &ListedSource) -> String {
    match source {
        ListedSource::Github { repo, reference } => {
            format!("GitHub ({})", with_ref(repo, reference.as_ref()))
        }
        ListedSource::Git { url, reference } => {
            format!("Git ({})", with_ref(url, reference.as_ref()))
        }
        ListedSource::Url { url } => format!("URL ({url})"),
        ListedSource::Directory { path } => format!("Directory ({path})"),
        ListedSource::File { path } => format!("File ({path})"),
        ListedSource::Claudeai { .. } => "claude.ai".to_owned(),
    }
}

/// `installLocation` for a cached marketplace; `scope` and `status` when a claude.ai entry records them.
fn place_fields(marketplace: &ConfiguredMarketplace) -> ListedJson {
    let mut json = ListedJson::new();
    if let Some(location) = &marketplace.install_location {
        json.insert("installLocation".to_owned(), location.clone());
    }
    if let Some(scope) = &marketplace.scope {
        json.insert("scope".to_owned(), scope.clone());
    }
    if let Some(status) = &marketplace.status {
        json.insert("status".to_owned(), status.clone());
    }
    json
}

fn json_of(marketplace: &ConfiguredMarketplace) -> ListedJson {
    let mut json = fields(&[("name", &marketplace.name)]);
    json.extend(fields_of(&marketplace.source));
    json.extend(place_fields(marketplace));
    json
}

fn entry_lines(marketplace: &ConfiguredMarketplace) -> Vec<String> {
    vec![
        format!("  {}", marketplace.name),
        format!("    Source: {}", describe_source(&marketplace.source)),
    ]
}

fn text_of(request: &ListRequest) -> Vec<String> {
    let mut lines = if request.marketplaces.is_empty() {
        vec!["No marketplaces configured".to_owned()]
    } else {
        let mut head = vec!["Configured marketplaces:".to_owned()];
        head.extend(request.marketplaces.iter().flat_map(entry_lines));
        head
    };
    let hosted = request.from_claude_ai.as_deref().unwrap_or_default();
    if !hosted.is_empty() {
        lines.push("From claude.ai:".to_owned());
        lines.extend(hosted.iter().map(|one| format!("  {}", one.name)));
    }
    lines
}

/// The listing: `--json` covers "configured marketplaces only and leaves the section out".
#[must_use]
pub fn plan_marketplace_list(request: &ListRequest) -> ListPlan {
    let json: Vec<ListedJson> = request.marketplaces.iter().map(json_of).collect();
    let text = text_of(request);
    let print = Function::new(Print {
        json: json.clone(),
        text: text.clone(),
    });
    ListPlan { json, text, print }
}

crate::registry::exports! {
    "cli/marketplace-list" {
        "planMarketplaceList" => ok |request: ListRequest| plan_marketplace_list(&request);
    }
}
