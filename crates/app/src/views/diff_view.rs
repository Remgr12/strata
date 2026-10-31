use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;
use strata_core::SqlDialect;

pub struct DiffView {
    pub state: Entity<AppState>,
}

impl DiffView {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let has_snapshot = app_state.snapshot_schema.is_some();
        let diff = app_state.schema_diff.as_ref();
        let dialect = app_state.diff_dialect;
        let migration_sql = app_state.diff_migration_sql.join("\n");

        let added_count = diff.map(|d| d.added_tables.len()).unwrap_or(0);
        let dropped_count = diff.map(|d| d.dropped_tables.len()).unwrap_or(0);
        let altered_count = diff.map(|d| d.altered_tables.len()).unwrap_or(0);

        div()
            .flex_1()
            .flex()
            .flex_col()
            .bg(rgb(0x181825))
            .child(
                // Diff Toolbar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x1e1e2e))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0xcdd6f4))
                                    .child("SCHEMA COMPARISON & MIGRATION GENERATOR"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0xa6e3a1))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x11111b))
                                    .child(format!("+{} Added", added_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0xf38ba8))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x11111b))
                                    .child(format!("-{} Dropped", dropped_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0xf9e2af))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x11111b))
                                    .child(format!("~{} Altered", altered_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Snapshot trigger button
                            .child(
                                div()
                                    .id("btn-take-snapshot")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x89b4fa))
                                    .hover(|s| s.bg(rgb(0xb4befe)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x11111b))
                                    .child(if has_snapshot {
                                        "Re-take Snapshot"
                                    } else {
                                        "Take Schema Snapshot"
                                    })
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.take_schema_snapshot(cx);
                                            });
                                        }
                                    })),
                            )
                            // Dialect toggles
                            .child(
                                div()
                                    .id("btn-dialect-sqlite")
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(dialect == SqlDialect::Sqlite, |el| el.bg(rgb(0x3e8fb0)))
                                    .when(dialect != SqlDialect::Sqlite, |el| el.bg(rgb(0x313244)))
                                    .text_xs()
                                    .text_color(rgb(0xcdd6f4))
                                    .child("SQLite")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.set_diff_dialect(SqlDialect::Sqlite, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-dialect-duckdb")
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(dialect == SqlDialect::DuckDb, |el| el.bg(rgb(0xf6c177)))
                                    .when(dialect != SqlDialect::DuckDb, |el| el.bg(rgb(0x313244)))
                                    .text_xs()
                                    .text_color(if dialect == SqlDialect::DuckDb {
                                        rgb(0x11111b)
                                    } else {
                                        rgb(0xcdd6f4)
                                    })
                                    .child("DuckDB")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.set_diff_dialect(SqlDialect::DuckDb, cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // Main Diff content (Visual Diff + Generated Migration SQL)
                div()
                    .flex_1()
                    .flex()
                    .child(
                        // Left Column: Visual Diff Tree
                        div()
                            .id("diff-visual-tree-scroll")
                            .flex_1()
                            .h_full()
                            .p_3()
                            .overflow_y_scroll()
                            .border_r_1()
                            .border_color(rgb(0x313244))
                            .child(if !has_snapshot {
                                div()
                                    .p_6()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0x6c7086))
                                    .child("Click 'Take Schema Snapshot' to establish a baseline.\nMake schema modifications or load another database to see the diff.")
                            } else if diff.map(|d| d.is_empty()).unwrap_or(true) {
                                div()
                                    .p_6()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0xa6e3a1))
                                    .child("✔ Schemas are identical. No differences detected.")
                            } else {
                                let mut diff_cards = Vec::new();
                                let d = diff.unwrap();

                                // Added tables (Green)
                                for tbl in &d.added_tables {
                                    diff_cards.push(
                                        div()
                                            .mb_2()
                                            .p_3()
                                            .rounded_md()
                                            .bg(rgb(0x1e3a2b))
                                            .border_1()
                                            .border_color(rgb(0x2d6a4f))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .font_weight(gpui::FontWeight::BOLD)
                                                            .text_xs()
                                                            .text_color(rgb(0xa6e3a1))
                                                            .child(format!("+ ADD TABLE: {}", tbl.name)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x94e2d5))
                                                    .child(format!("{} columns", tbl.columns.len())),
                                            ),
                                    );
                                }

                                // Dropped tables (Red)
                                for tbl in &d.dropped_tables {
                                    diff_cards.push(
                                        div()
                                            .mb_2()
                                            .p_3()
                                            .rounded_md()
                                            .bg(rgb(0x3c1f24))
                                            .border_1()
                                            .border_color(rgb(0x7f2334))
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_xs()
                                                    .text_color(rgb(0xf38ba8))
                                                    .child(format!("- DROP TABLE: {}", tbl.name)),
                                            ),
                                    );
                                }

                                // Altered tables (Yellow/Amber)
                                for alt in &d.altered_tables {
                                    let mut changes = Vec::new();

                                    for col in &alt.added_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xa6e3a1))
                                                .child(format!("+ add column: {} ({})", col.name, col.data_type.sql_repr())),
                                        );
                                    }

                                    for col in &alt.dropped_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xf38ba8))
                                                .child(format!("- drop column: {}", col.name)),
                                        );
                                    }

                                    for cd in &alt.altered_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xf9e2af))
                                                .child(format!(
                                                    "~ alter column: {} ({} -> {})",
                                                    cd.column_name,
                                                    cd.old_column.data_type.sql_repr(),
                                                    cd.new_column.data_type.sql_repr()
                                                )),
                                        );
                                    }

                                    diff_cards.push(
                                        div()
                                            .mb_2()
                                            .p_3()
                                            .rounded_md()
                                            .bg(rgb(0x382c16))
                                            .border_1()
                                            .border_color(rgb(0x6b4f1f))
                                            .child(
                                                div()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_xs()
                                                    .text_color(rgb(0xf9e2af))
                                                    .child(format!("~ ALTER TABLE: {}", alt.table_name)),
                                            )
                                            .children(changes),
                                    );
                                }

                                div().flex().flex_col().children(diff_cards)
                            }),
                    )
                    .child(
                        // Right Column: Declarative SQL Migration Output
                        div()
                            .flex_1()
                            .h_full()
                            .p_3()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .mb_2()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x89b4fa))
                                    .child("DECLARATIVE SQL MIGRATION SCRIPT"),
                            )
                            .child(
                                div()
                                    .id("diff-sql-script-scroll")
                                    .flex_1()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x11111b))
                                    .border_1()
                                    .border_color(rgb(0x313244))
                                    .overflow_y_scroll()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(rgb(0xa6adc8))
                                    .child(if migration_sql.is_empty() {
                                        "-- No schema migration statements needed".to_string()
                                    } else {
                                        migration_sql
                                    }),
                            ),
                    ),
            )
    }
}
