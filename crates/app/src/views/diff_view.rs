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
        let theme = app_state.theme.clone();
        let has_snapshot = app_state.snapshot_schema.is_some();
        let snapshot_label = app_state
            .snapshot_label
            .clone()
            .unwrap_or_else(|| "None".to_string());
        let diff = app_state.schema_diff.as_ref();
        let dialect = app_state.diff_dialect;
        let migration_lines = app_state.diff_migration_sql.clone();
        let migration_sql = migration_lines.join("\n\n");

        let added_count = diff.map(|d| d.added_tables.len()).unwrap_or(0);
        let dropped_count = diff.map(|d| d.dropped_tables.len()).unwrap_or(0);
        let altered_count = diff.map(|d| d.altered_tables.len()).unwrap_or(0);

        let sql_lines: Vec<String> = if migration_sql.is_empty() {
            vec!["-- No migration statements".to_string()]
        } else {
            migration_sql.lines().map(|s| s.to_string()).collect()
        };

        let mut numbered_sql_rows = Vec::new();
        for (idx, line) in sql_lines.iter().enumerate() {
            numbered_sql_rows.push(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_0p5()
                    .child(
                        div()
                            .w(px(24.0))
                            .text_right()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(theme.text_muted_rgb())
                            .child(format!("{}", idx + 1)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(if line.starts_with("--") {
                                theme.text_muted_rgb()
                            } else {
                                theme.text_primary_rgb()
                            })
                            .child(line.clone()),
                    ),
            );
        }

        div()
            .flex_1()
            .flex()
            .flex_col()
            .bg(theme.substrate_rgb())
            .child(
                // Material Diff Action Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2p5()
                    .bg(theme.surface_rgb())
                    .border_b_1()
                    .border_color(theme.outline_rgb())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary_rgb())
                                    .child(format!("Baseline: {}", snapshot_label)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(6.0))
                                    .bg(theme.telemetry_container_rgb())
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.telemetry_rgb())
                                    .child(format!("+{}", added_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(6.0))
                                    .bg(theme.danger_container_rgb())
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.danger_rgb())
                                    .child(format!("-{}", dropped_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(6.0))
                                    .bg(theme.accent_container_rgb())
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.accent_rgb())
                                    .child(format!("~{}", altered_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("btn-take-snapshot")
                                    .px_3()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(theme.accent_rgb())
                                    .hover({
                                        let h = theme.accent_hover_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.on_accent_rgb())
                                    .child("Snapshot")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.take_schema_snapshot(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-compare-file")
                                    .px_3()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(theme.surface_container_rgb())
                                    .hover({
                                        let h = theme.surface_container_high_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.text_primary_rgb())
                                    .child("Compare")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.prompt_compare_file(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .p_0p5()
                                    .rounded(px(8.0))
                                    .bg(theme.surface_container_rgb())
                                    .child(
                                        div()
                                            .id("btn-dialect-sqlite")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .when(dialect == SqlDialect::Sqlite, |el| {
                                                el.bg(theme.primary_rgb())
                                                    .text_color(theme.on_primary_rgb())
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                            })
                                            .when(dialect != SqlDialect::Sqlite, |el| {
                                                el.text_color(theme.text_secondary_rgb())
                                            })
                                            .text_xs()
                                            .child("SQLite")
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                move |_this, _event, _window, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.set_diff_dialect(
                                                            SqlDialect::Sqlite,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("btn-dialect-duckdb")
                                            .px_2p5()
                                            .py_0p5()
                                            .rounded(px(6.0))
                                            .cursor_pointer()
                                            .when(dialect == SqlDialect::DuckDb, |el| {
                                                el.bg(theme.primary_rgb())
                                                    .text_color(theme.on_primary_rgb())
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                            })
                                            .when(dialect != SqlDialect::DuckDb, |el| {
                                                el.text_color(theme.text_secondary_rgb())
                                            })
                                            .text_xs()
                                            .child("DuckDB")
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                move |_this, _event, _window, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.set_diff_dialect(
                                                            SqlDialect::DuckDb,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                // Split Pane
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(
                        // Left Column: Structural Changes
                        div()
                            .id("diff-visual-tree-scroll")
                            .flex_1()
                            .h_full()
                            .p_4()
                            .overflow_y_scroll()
                            .border_r_1()
                            .border_color(theme.outline_rgb())
                            .child(if !has_snapshot {
                                div()
                                    .p_6()
                                    .rounded(px(10.0))
                                    .bg(theme.surface_rgb())
                                    .border_1()
                                    .border_color(theme.outline_rgb())
                                    .text_center()
                                    .text_xs()
                                    .text_color(theme.text_muted_rgb())
                                    .child("No baseline snapshot")
                            } else if diff.map(|d| d.is_empty()).unwrap_or(true) {
                                div()
                                    .p_6()
                                    .rounded(px(10.0))
                                    .bg(theme.surface_rgb())
                                    .border_1()
                                    .border_color(theme.outline_rgb())
                                    .text_center()
                                    .text_xs()
                                    .text_color(theme.telemetry_rgb())
                                    .child("Schemas match")
                            } else {
                                let mut diff_cards = Vec::new();
                                let d = diff.unwrap();

                                for tbl in &d.added_tables {
                                    let mut col_pills = Vec::new();
                                    for col in &tbl.columns {
                                        col_pills.push(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .py_0p5()
                                                .text_xs()
                                                .child(
                                                    div()
                                                        .text_color(theme.text_primary_rgb())
                                                        .child(format!("+ {}", col.name)),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("monospace")
                                                        .text_color(theme.telemetry_rgb())
                                                        .child(col.data_type.sql_repr()),
                                                ),
                                        );
                                    }

                                    diff_cards.push(
                                        div()
                                            .mb_3()
                                            .p_3p5()
                                            .rounded(px(10.0))
                                            .bg(theme.surface_rgb())
                                            .border_1()
                                            .border_color(theme.telemetry_rgb())
                                            .flex()
                                            .flex_col()
                                            .gap_1p5()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                            .text_color(theme.telemetry_rgb())
                                                            .child(format!("+ {}", tbl.name)),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(theme.text_muted_rgb())
                                                            .child(format!("{} cols", tbl.columns.len())),
                                                    ),
                                            )
                                            .children(col_pills),
                                    );
                                }

                                for tbl in &d.dropped_tables {
                                    diff_cards.push(
                                        div()
                                            .mb_3()
                                            .p_3p5()
                                            .rounded(px(10.0))
                                            .bg(theme.surface_rgb())
                                            .border_1()
                                            .border_color(theme.danger_rgb())
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(theme.danger_rgb())
                                                    .child(format!("- {}", tbl.name)),
                                            ),
                                    );
                                }

                                for alt in &d.altered_tables {
                                    let mut changes = Vec::new();

                                    for col in &alt.added_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(theme.telemetry_rgb())
                                                .child(format!(
                                                    "+ {} ({})",
                                                    col.name,
                                                    col.data_type.sql_repr()
                                                )),
                                        );
                                    }

                                    for col in &alt.dropped_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(theme.danger_rgb())
                                                .child(format!("- {}", col.name)),
                                        );
                                    }

                                    for cd in &alt.altered_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(theme.accent_rgb())
                                                .child(format!(
                                                    "~ {}: {} → {}",
                                                    cd.column_name,
                                                    cd.old_column.data_type.sql_repr(),
                                                    cd.new_column.data_type.sql_repr()
                                                )),
                                        );
                                    }

                                    diff_cards.push(
                                        div()
                                            .mb_3()
                                            .p_3p5()
                                            .rounded(px(10.0))
                                            .bg(theme.surface_rgb())
                                            .border_1()
                                            .border_color(theme.accent_rgb())
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(theme.accent_rgb())
                                                    .child(format!("~ {}", alt.table_name)),
                                            )
                                            .children(changes),
                                    );
                                }

                                div().flex().flex_col().children(diff_cards)
                            }),
                    )
                    .child(
                        // Right Column: Migration DDL Preview
                        div()
                            .flex_1()
                            .h_full()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_2p5()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(theme.text_secondary_rgb())
                                            .child("DDL"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .id("btn-copy-migration")
                                                    .px_2p5()
                                                    .py_1()
                                                    .rounded(px(6.0))
                                                    .bg(theme.surface_container_rgb())
                                                    .hover({
                                                        let h = theme.surface_container_high_rgb();
                                                        move |s| s.bg(h)
                                                    })
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(theme.text_secondary_rgb())
                                                    .child("Copy")
                                                    .on_click(cx.listener({
                                                        let state_model = self.state.clone();
                                                        move |_this, _event, _window, cx| {
                                                            let sql = state_model
                                                                .read(cx)
                                                                .diff_migration_sql
                                                                .join("\n\n");
                                                            cx.write_to_clipboard(
                                                                ClipboardItem::new_string(sql),
                                                            );
                                                        }
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .id("btn-open-in-editor")
                                                    .px_3()
                                                    .py_1()
                                                    .rounded(px(6.0))
                                                    .bg(theme.primary_rgb())
                                                    .hover({
                                                        let h = theme.primary_hover_rgb();
                                                        move |s| s.bg(h)
                                                    })
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(theme.on_primary_rgb())
                                                    .child("Open in Editor")
                                                    .on_click(cx.listener({
                                                        let state_model = self.state.clone();
                                                        move |_this, _event, _window, cx| {
                                                            state_model.update(
                                                                cx,
                                                                |state, cx| {
                                                                    state.load_migration_into_editor(cx);
                                                                },
                                                            );
                                                        }
                                                    })),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id("diff-sql-script-scroll")
                                    .flex_1()
                                    .p_3p5()
                                    .rounded(px(10.0))
                                    .bg(theme.surface_lowest_rgb())
                                    .border_1()
                                    .border_color(theme.outline_rgb())
                                    .overflow_y_scroll()
                                    .children(numbered_sql_rows),
                            ),
                    ),
            )
    }
}
