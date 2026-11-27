//! Port of `src/marketplace/marketplace-catalog.ts`, switched.
//!
//! Reading a marketplace's catalog once the host has it (plugins/marketplace-reference.md, "Fields by
//! type"): the text of a `url` download or a `file` source, or a `directory` source, "the directory
//! that contains `.claude-plugin/marketplace.json`". Both are `readMarketplace` over the parsed file;
//! invalid JSON or a missing file keeps the marketplace from loading, never a throw.

use crate::format::issues::error;
use crate::format::marketplace::marketplace::{EntryReader, read_marketplace};
use crate::format::marketplace::types::{MarketplaceContext, MarketplaceResult};
use crate::format::read_json::{JsonRead, invalid_json_syntax, parse_json};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// Where a marketplace keeps its catalog.
pub const MARKETPLACE_FILE: &str = ".claude-plugin/marketplace.json";

/// The marketplace a `marketplace.json` text holds.
pub async fn read_marketplace_text(text: &str, context: &MarketplaceContext) -> MarketplaceResult {
    match parse_json(text) {
        JsonRead::Value { value } => read_marketplace(&value, context, EntryReader::Default).await,
        JsonRead::Problem { problem, .. } => MarketplaceResult {
            marketplace: None,
            issues: vec![error(MARKETPLACE_FILE, &invalid_json_syntax(&problem))],
        },
    }
}

/// The marketplace of a directory, through `files` rooted at it.
pub async fn read_marketplace_directory(
    files: &dyn PluginFiles,
    context: &MarketplaceContext,
    signal: Option<&Signal>,
) -> MarketplaceResult {
    match files.read_text(MARKETPLACE_FILE, signal).await {
        None => MarketplaceResult {
            marketplace: None,
            issues: vec![error(MARKETPLACE_FILE, "The marketplace file is missing")],
        },
        Some(text) => read_marketplace_text(&text, context).await,
    }
}

crate::registry::exports! {
    "marketplace/marketplace-catalog" {
        "MARKETPLACE_FILE" => ok || MARKETPLACE_FILE;
        "readMarketplaceText" => async ok |text: String, context: Option<MarketplaceContext>|
            read_marketplace_text(&text, &context.unwrap_or_default()).await;
        "readMarketplaceDirectory" => async ok |files: Port, context: Option<MarketplaceContext>, signal: Option<Signal>|
            read_marketplace_directory(&files, &context.unwrap_or_default(), signal.as_ref()).await;
    }
}
