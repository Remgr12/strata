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
            vec!["-- No migration statements required".to_string()]
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
                            .text_color(rgb(0x43495c))
                            .child(format!("{}", idx + 1)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(if line.starts_with("--") {
                                rgb(0x5c637a)
                            } else {
                                rgb(0xeceff4)
                            })
                            .child(line.clone()),
                    ),
            );
        }

        div()
            .flex_1()
            .flex()
            .flex_col()
            .bg(rgb(0x090a0f))
            .child(
                // Diff Action Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2p5()
                    .bg(rgb(0x0f1117))
                    .border_b_1()
                    .border_color(rgb(0x1e2230))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xeceff4))
                                    .child(format!("Baseline: {}", snapshot_label)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x112820))
                                    .text_xs()
                                    .text_color(rgb(0x3dd68c))
                                    .child(format!("+{} added", added_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x281218))
                                    .text_xs()
                                    .text_color(rgb(0xf2777a))
                                    .child(format!("-{} dropped", dropped_count)),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x282012))
                                    .text_xs()
                                    .text_color(rgb(0xf5a623))
                                    .child(format!("~{} modified", altered_count)),
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
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(5.0))
                                    .bg(rgb(0x5e81f4))
                                    .hover(|s| s.bg(rgb(0x7393f7)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(0xffffff))
                                    .child(if has_snapshot {
                                        "Update Snapshot"
                                    } else {
                                        "Capture Snapshot"
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
                            .child(
                                div()
                                    .id("btn-compare-file")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(5.0))
                                    .bg(rgb(0x181c26))
                                    .border_1()
                                    .border_color(rgb(0x252a3a))
                                    .hover(|s| s.bg(rgb(0x212634)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0xc8d0e0))
                                    .child("Compare File...")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.prompt_compare_file(cx);
                                            });
                                        }
                                    })),
                            )
                            // Dialect Segmented Toggle
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .p_0p5()
                                    .rounded(px(5.0))
                                    .bg(rgb(0x141720))
                                    .border_1()
                                    .border_color(rgb(0x1e2230))
                                    .child(
                                        div()
                                            .id("btn-dialect-sqlite")
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .when(dialect == SqlDialect::Sqlite, |el| {
                                                el.bg(rgb(0x232838)).text_color(rgb(0xffffff))
                                            })
                                            .when(dialect != SqlDialect::Sqlite, |el| {
                                                el.text_color(rgb(0x949cbb))
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
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .when(dialect == SqlDialect::DuckDb, |el| {
                                                el.bg(rgb(0x232838)).text_color(rgb(0xffffff))
                                            })
                                            .when(dialect != SqlDialect::DuckDb, |el| {
                                                el.text_color(rgb(0x949cbb))
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
                // Split Pane: Structural Changes (Left) & Generated SQL (Right)
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
                            .border_color(rgb(0x1e2230))
                            .child(if !has_snapshot {
                                div()
                                    .p_6()
                                    .rounded(px(8.0))
                                    .bg(rgb(0x0f1117))
                                    .border_1()
                                    .border_color(rgb(0x1e2230))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xeceff4))
                                            .child("No baseline snapshot selected"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_center()
                                            .text_color(rgb(0x5c637a))
                                            .child("Click 'Capture Snapshot' to record the current schema, or 'Compare File...' to diff against another database."),
                                    )
                            } else if diff.map(|d| d.is_empty()).unwrap_or(true) {
                                div()
                                    .p_6()
                                    .rounded(px(8.0))
                                    .bg(rgb(0x0f1117))
                                    .border_1()
                                    .border_color(rgb(0x1e2230))
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0x3dd68c))
                                    .child("Schemas are identical — no structural differences detected.")
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
                                                        .text_color(rgb(0xc8d0e0))
                                                        .child(format!("+ {}", col.name)),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("monospace")
                                                        .text_color(rgb(0x5c637a))
                                                        .child(col.data_type.sql_repr()),
                                                ),
                                        );
                                    }

                                    diff_cards.push(
                                        div()
                                            .mb_2p5()
                                            .p_3()
                                            .rounded(px(6.0))
                                            .bg(rgb(0x0f1916))
                                            .border_1()
                                            .border_color(rgb(0x1c3b2f))
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
                                                            .text_color(rgb(0x3dd68c))
                                                            .child(format!("Added Table: {}", tbl.name)),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x5c637a))
                                                            .child(format!("{} columns", tbl.columns.len())),
                                                    ),
                                            )
                                            .children(col_pills),
                                    );
                                }

                                for tbl in &d.dropped_tables {
                                    diff_cards.push(
                                        div()
                                            .mb_2p5()
                                            .p_3()
                                            .rounded(px(6.0))
                                            .bg(rgb(0x1b1014))
                                            .border_1()
                                            .border_color(rgb(0x3d1c24))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0xf2777a))
                                                    .child(format!("Dropped Table: {}", tbl.name)),
                                            ),
                                    );
                                }

                                for alt in &d.altered_tables {
                                    let mut changes = Vec::new();

                                    for col in &alt.added_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x3dd68c))
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
                                                .text_color(rgb(0xf2777a))
                                                .child(format!("- {}", col.name)),
                                        );
                                    }

                                    for cd in &alt.altered_columns {
                                        changes.push(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xf5a623))
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
                                            .mb_2p5()
                                            .p_3()
                                            .rounded(px(6.0))
                                            .bg(rgb(0x19150f))
                                            .border_1()
                                            .border_color(rgb(0x3b2e19))
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0xf5a623))
                                                    .child(format!(
                                                        "Modified Table: {}",
                                                        alt.table_name
                                                    )),
                                            )
                                            .children(changes),
                                    );
                                }

                                div().flex().flex_col().children(diff_cards)
                            }),
                    )
                    .child(
                        // Right Column: Migration SQL Preview & Actions
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
                                            .text_color(rgb(0x949cbb))
                                            .child("Migration SQL Script"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .child(
                                                div()
                                                    .id("btn-copy-migration")
                                                    .px_2()
                                                    .py_1()
                                                    .rounded(px(4.0))
                                                    .bg(rgb(0x141720))
                                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(rgb(0xc8d0e0))
                                                    .child("Copy SQL")
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
                                                            state_model.update(
                                                                cx,
                                                                |state, cx| {
                                                                    state.status_message = Some(
                                                                        "Copied migration SQL to clipboard"
                                                                            .to_string(),
                                                                    );
                                                                    cx.notify();
                                                                },
                                                            );
                                                        }
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .id("btn-open-in-editor")
                                                    .px_2p5()
                                                    .py_1()
                                                    .rounded(px(4.0))
                                                    .bg(rgb(0x5e81f4))
                                                    .hover(|s| s.bg(rgb(0x7393f7)))
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .text_color(rgb(0xffffff))
                                                    .child("Open in SQL Editor")
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
                                    .p_3()
                                    .rounded(px(6.0))
                                    .bg(rgb(0x0f1117))
                                    .border_1()
                                    .border_color(rgb(0x1e2230))
                                    .overflow_y_scroll()
                                    .children(numbered_sql_rows),
                            ),
                    ),
            )
    }
}
