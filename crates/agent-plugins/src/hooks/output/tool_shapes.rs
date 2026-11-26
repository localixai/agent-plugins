//! Written by `bun scripts/tool-output-shapes.ts` from the Agent SDK `sdk-tools.d.ts`; do not edit.
//!
//! The output shape of every built-in tool the SDK types, by tool name: the tables of `tool_shapes/`
//! joined, a tool split over several holding every alternative.

mod shapes_0;
mod shapes_1;
mod shapes_2;
mod shapes_3;
mod shapes_4;
mod shapes_5;

use std::sync::LazyLock;

use super::tool_shape::{ShapeTable, join_tables, read_table};

/// The tables' JSON texts, in order.
const TABLES: [&str; 6] = [
    shapes_0::SHAPES,
    shapes_1::SHAPES,
    shapes_2::SHAPES,
    shapes_3::SHAPES,
    shapes_4::SHAPES,
    shapes_5::SHAPES,
];

/// The output shape of every built-in tool the SDK types, by tool name.
pub static TOOL_SHAPES: LazyLock<ShapeTable> =
    LazyLock::new(|| join_tables(&TABLES.map(read_table)));
