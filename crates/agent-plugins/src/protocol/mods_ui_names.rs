//! Port of `src/protocol/mods-ui-names.ts`, switched.
//!
//! What the mods reference says about the interface, as data (plugins/mods/reference.md, "Render
//! sites" and "Elements"): the render sites with the fields of `e.props`, what `e.requestId` is and the
//! apps that raise them; the elements with the apps that draw them, the props the VS Code extension
//! ignores, the `Box` border styles and spacing steps, and the `Text` values. Pure data.

use indexmap::IndexMap;
use serde::Serialize;

/// The apps that raise a render site.
pub const UI_SURFACES: &[&str] = &["terminal", "desktop", "vscode", "mobile"];

/// The apps of the elements table: its three columns.
pub const ELEMENT_COLUMNS: &[&str] = &["terminal", "desktop", "vscode"];

/// The column of the elements table each app draws by. The table has no column for the mobile app,
/// which the docs name only as an app that raises `Backdrop`; the kit takes it to draw what the
/// Desktop app draws, with the same props, but no `Client` (`drawsElement`; judgment,
/// spec/judgments.md).
#[must_use]
pub fn element_column_of() -> IndexMap<&'static str, &'static str> {
    IndexMap::from([
        ("terminal", "terminal"),
        ("desktop", "desktop"),
        ("vscode", "vscode"),
        ("mobile", "desktop"),
    ])
}

/// What `e.requestId` of a `ui.render` event is.
pub const REQUEST_KINDS: &[&str] = &[
    "pane-id",
    "instance",
    "message-id",
    "tool-call-id",
    "agent-id",
    "placed-id",
    "backdrop",
];

/// One render site of the reference's table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSiteDef {
    /// The fields of `e.props` the table names. A row that describes its props in words ("The
    /// reply's text", "The tool's name, input, and result") names none: the list is then empty
    /// (judgment, spec/judgments.md).
    pub props: Vec<&'static str>,
    /// What `e.requestId` is.
    pub request: &'static str,
    /// The apps that raise it.
    pub surfaces: Vec<&'static str>,
}

fn site(props: &[&'static str], request: &'static str, surfaces: &[&'static str]) -> RenderSiteDef {
    RenderSiteDef {
        props: props.to_vec(),
        request,
        surfaces: surfaces.to_vec(),
    }
}

const ALL: &[&str] = &["terminal", "desktop", "vscode"];
const TERMINAL_AND_DESKTOP: &[&str] = &["terminal", "desktop"];
const TERMINAL: &[&str] = &["terminal"];
/// The Desktop app and the mobile app: the only apps that raise `Backdrop`.
const DESKTOP_AND_MOBILE: &[&str] = &["desktop", "mobile"];

/// `e.requestId` of the `Backdrop` site: one backdrop for each session.
pub const BACKDROP_REQUEST_ID: &str = "backdrop";

/// The render sites of the reference's table, in its order.
#[must_use]
pub fn render_sites() -> IndexMap<&'static str, RenderSiteDef> {
    IndexMap::from([
        (
            "Pane",
            site(
                &[
                    "title",
                    "isFocused",
                    "bodyColumns",
                    "placement",
                    "scroll",
                    "view",
                ],
                "pane-id",
                ALL,
            ),
        ),
        (
            "AbovePrompt",
            site(
                &[
                    "hasSurvey",
                    "isWorking",
                    "maxRows",
                    "bodyColumns",
                    "scroll",
                    "view",
                ],
                "instance",
                TERMINAL_AND_DESKTOP,
            ),
        ),
        // A row `$.ui.place` put in the transcript: its `requestId` is the one the call returned.
        (
            "Placed",
            site(&["id", "plugin", "onScreen"], "placed-id", TERMINAL),
        ),
        // The session's backdrop (`$.theme.set`): its `requestId` is always `backdrop`.
        (
            "Backdrop",
            site(
                &["id", "plugin", "glass", "colorScheme"],
                "backdrop",
                DESKTOP_AND_MOBILE,
            ),
        ),
        (
            "UserMessage",
            site(
                &["text", "origin", "isExpanded", "task", "from"],
                "message-id",
                ALL,
            ),
        ),
        ("AssistantMessage", site(&[], "message-id", ALL)),
        ("ToolUse", site(&[], "tool-call-id", ALL)),
        ("ToolResult", site(&[], "tool-call-id", ALL)),
        ("ToolGroup", site(&[], "tool-call-id", ALL)),
        (
            "CommandOutput",
            site(&["command", "text"], "message-id", TERMINAL_AND_DESKTOP),
        ),
        ("AskUserQuestion", site(&[], "tool-call-id", ALL)),
        ("ToolProgress", site(&["kind"], "tool-call-id", TERMINAL)),
        (
            "Spinner",
            site(&["word", "message", "suffix", "mode"], "agent-id", ALL),
        ),
        (
            "TurnDuration",
            site(&["word", "durationMs"], "message-id", TERMINAL),
        ),
        (
            "InfoNotice",
            site(&["text", "command"], "message-id", TERMINAL),
        ),
        (
            "SessionMode",
            site(&["modes"], "instance", TERMINAL_AND_DESKTOP),
        ),
        (
            "PromptHint",
            site(
                &["isDraft", "isWorking", "hint"],
                "instance",
                TERMINAL_AND_DESKTOP,
            ),
        ),
    ])
}

/// The sites, in the order of the reference table.
pub const RENDER_SITE_NAMES: &[&str] = &[
    "Pane",
    "AbovePrompt",
    "Placed",
    "Backdrop",
    "UserMessage",
    "AssistantMessage",
    "ToolUse",
    "ToolResult",
    "ToolGroup",
    "CommandOutput",
    "AskUserQuestion",
    "ToolProgress",
    "Spinner",
    "TurnDuration",
    "InfoNotice",
    "SessionMode",
    "PromptHint",
];

/// The elements of the reference, in its order.
pub const ELEMENT_NAMES: &[&str] = &[
    "Box", "Text", "Button", "Link", "Code", "Markdown", "Input", "Select", "Svg", "Client",
    "Raster", "Image",
];

/// The apps that draw each element (the check marks of the elements table).
#[must_use]
pub fn element_surfaces() -> IndexMap<&'static str, Vec<&'static str>> {
    let table: [(&str, &[&str]); 12] = [
        ("Box", ALL),
        ("Text", ALL),
        ("Button", ALL),
        ("Link", ALL),
        ("Code", ALL),
        ("Markdown", ALL),
        ("Input", ALL),
        ("Select", ALL),
        ("Svg", &["desktop", "vscode"]),
        ("Client", TERMINAL_AND_DESKTOP),
        ("Raster", TERMINAL),
        ("Image", TERMINAL),
    ];
    table
        .into_iter()
        .map(|(name, surfaces)| (name, surfaces.to_vec()))
        .collect()
}

/// The elements an app leaves out though its column of the table draws them: the mobile app takes
/// the Desktop column but has no `Client` (the declarations' mobile `Elements` table,
/// claude-code/index.d.ts:4108; Agent Plugins Extended §10.27, Judgment J10.104).
fn not_drawn_by(surface: &str) -> &'static [&'static str] {
    match surface {
        "mobile" => &["Client"],
        _ => &[],
    }
}

/// Whether `surface` draws the element `name`: its column's check mark, less what the app leaves out.
#[must_use]
pub fn draws_element(name: &str, surface: &str) -> bool {
    let column = element_column_of().get(surface).copied();
    let drawn = element_surfaces()
        .get(name)
        .is_some_and(|columns| column.is_some_and(|one| columns.contains(&one)));
    drawn && !not_drawn_by(surface).contains(&name)
}

/// The props the VS Code extension ignores: it draws the element as if they were left out
/// (reference.md, "Elements"). The terminal and the Desktop app ignore none the docs name.
#[must_use]
pub fn vscode_ignored() -> IndexMap<&'static str, Vec<&'static str>> {
    let table: [(&str, &[&str]); 12] = [
        (
            "Box",
            &[
                "position",
                "top",
                "left",
                "right",
                "bottom",
                "alignSelf",
                "borderDimColor",
                "hover",
            ],
        ),
        ("Text", &["size", "mono", "tabular", "hover"]),
        (
            "Button",
            &[
                "hotkey",
                "plain",
                "autoFocus",
                "action",
                "hover",
                "variant",
                "role",
            ],
        ),
        ("Link", &[]),
        ("Code", &["language", "path", "startLine", "format", "wrap"]),
        ("Markdown", &[]),
        ("Input", &["autoFocus", "rows"]),
        ("Select", &["autoFocus"]),
        ("Svg", &["isInteractive"]),
        ("Client", &[]),
        ("Raster", &[]),
        ("Image", &[]),
    ];
    table
        .into_iter()
        .map(|(name, props)| (name, props.to_vec()))
        .collect()
}

/// `Box` `borderStyle` names; any other value draws no border (reference.md, "`Box` border styles").
pub const BORDER_STYLES: &[&str] = &[
    "single",
    "double",
    "round",
    "bold",
    "singleDouble",
    "doubleSingle",
    "classic",
    "arrow",
    "dashed",
    "quote",
];

/// What one spacing step is, in terminal cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SpacingStep {
    /// Cells across.
    pub across: u32,
    /// Cells down.
    pub down: u32,
}

/// What each spacing step is, in terminal cells (reference.md, "`Box` spacing steps").
#[must_use]
pub fn spacing_steps() -> IndexMap<&'static str, SpacingStep> {
    IndexMap::from([
        ("xs", SpacingStep { across: 1, down: 0 }),
        ("sm", SpacingStep { across: 1, down: 0 }),
        ("md", SpacingStep { across: 2, down: 1 }),
        ("lg", SpacingStep { across: 3, down: 1 }),
        ("xl", SpacingStep { across: 4, down: 2 }),
    ])
}

/// The `Box` `flexDirection` values.
pub const FLEX_DIRECTIONS: &[&str] = &["row", "column", "row-reverse", "column-reverse"];

/// The `Text` `wrap` values.
pub const TEXT_WRAPS: &[&str] = &[
    "wrap",
    "truncate",
    "truncate-start",
    "truncate-middle",
    "truncate-end",
];

/// The `Text` `size` values.
pub const TEXT_SIZES: &[&str] = &["caption", "footnote", "body", "heading", "title"];

/// The `Text` `weight` values.
pub const TEXT_WEIGHTS: &[&str] = &["regular", "medium", "semibold", "bold"];

/// The `Button` `variant` values.
pub const BUTTON_VARIANTS: &[&str] = &["primary", "secondary"];

/// The `Button` `role` values.
pub const BUTTON_ROLES: &[&str] = &["dismiss"];

crate::registry::exports! {
    "protocol/mods-ui-names" {
        "UI_SURFACES" => ok || UI_SURFACES;
        "ELEMENT_COLUMNS" => ok || ELEMENT_COLUMNS;
        "ELEMENT_COLUMN_OF" => ok element_column_of;
        "REQUEST_KINDS" => ok || REQUEST_KINDS;
        "BACKDROP_REQUEST_ID" => ok || BACKDROP_REQUEST_ID;
        "RENDER_SITES" => ok render_sites;
        "RENDER_SITE_NAMES" => ok || RENDER_SITE_NAMES;
        "ELEMENT_NAMES" => ok || ELEMENT_NAMES;
        "ELEMENT_SURFACES" => ok element_surfaces;
        "drawsElement" => ok |name: String, surface: String| draws_element(&name, &surface);
        "VSCODE_IGNORED" => ok vscode_ignored;
        "BORDER_STYLES" => ok || BORDER_STYLES;
        "SPACING_STEPS" => ok spacing_steps;
        "FLEX_DIRECTIONS" => ok || FLEX_DIRECTIONS;
        "TEXT_WRAPS" => ok || TEXT_WRAPS;
        "TEXT_SIZES" => ok || TEXT_SIZES;
        "TEXT_WEIGHTS" => ok || TEXT_WEIGHTS;
        "BUTTON_VARIANTS" => ok || BUTTON_VARIANTS;
        "BUTTON_ROLES" => ok || BUTTON_ROLES;
    }
}
