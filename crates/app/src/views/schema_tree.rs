use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;

pub struct SchemaTreeView {
    pub state: Entity<AppState>,
    pub collapsed_tables: std::collections::HashSet<String>,
}

impl SchemaTreeView {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            collapsed_tables: std::collections::HashSet::new(),
        }
    }

    pub fn toggle_table_collapse(&mut self, table_name: &str) {
        if self.collapsed_tables.contains(table_name) {
            self.collapsed_tables.remove(table_name);
        } else {
            self.collapsed_tables.insert(table_name.to_string());
        }
    }
}

impl Render for SchemaTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let engine_type = app_state.engine_type.clone();
        let file_path = app_state
            .file_path
            .clone()
            .unwrap_or_else(|| "No file loaded".to_string());
        let tables = app_state
            .schema
            .as_ref()
            .map(|s| s.tables.clone())
            .unwrap_or_default();
        let selected_table = app_state.selected_table.clone();
        let is_loading = app_state.is_loading_schema;

        let badge_bg = match engine_type.as_str() {
            "SQLite" => rgb(0x3e8fb0),
            "DuckDB" => rgb(0xf6c177),
            "Parquet" => rgb(0x9ccfd8),
            _ => rgb(0x524f67),
        };

        div()
            .flex()
            .flex_col()
            .w(px(280.0))
            .h_full()
            .bg(rgb(0x181825))
            .border_r_1()
            .border_color(rgb(0x313244))
            .child(
                // Header / Connection info
                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0xcdd6f4))
                                    .child("EXPLORER"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(badge_bg)
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x11111b))
                                    .child(engine_type.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xa6adc8))
                            .overflow_hidden()
                            .child(file_path),
                    )
                    .child(
                        // Quick in-memory loader buttons
                        div()
                            .flex()
                            .gap_1()
                            .mt_1()
                            .child(
                                div()
                                    .id("btn-quick-sqlite")
                                    .flex_1()
                                    .p_1()
                                    .rounded_md()
                                    .bg(rgb(0x313244))
                                    .hover(|s| s.bg(rgb(0x45475a)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0x89b4fa))
                                    .child("+ SQLite")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.load_file(":memory:".to_string(), cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-quick-duckdb")
                                    .flex_1()
                                    .p_1()
                                    .rounded_md()
                                    .bg(rgb(0x313244))
                                    .hover(|s| s.bg(rgb(0x45475a)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0xf9e2af))
                                    .child("+ DuckDB")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.load_file(
                                                    "sample.duckdb".to_string(),
                                                    cx,
                                                );
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // Tables Tree Section
                div()
                    .id("tables-tree-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .child(if is_loading {
                        div()
                            .p_3()
                            .text_center()
                            .text_xs()
                            .text_color(rgb(0xf9e2af))
                            .child("Loading schema...")
                    } else if tables.is_empty() {
                        div()
                            .p_4()
                            .text_center()
                            .text_xs()
                            .text_color(rgb(0x6c7086))
                            .child("No tables found.\nDrag & drop a database file.")
                    } else {
                        let mut table_nodes = Vec::new();
                        for tbl in &tables {
                            let tbl_name = tbl.name.clone();
                            let is_selected = selected_table.as_deref() == Some(&tbl_name);
                            let cols = tbl.columns.clone();

                            let mut col_nodes = Vec::new();
                            for col in cols {
                                let type_repr = col.data_type.sql_repr();
                                col_nodes.push(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .pl_5()
                                        .pr_2()
                                        .py_0p5()
                                        .text_xs()
                                        .text_color(rgb(0xa6adc8))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(col.name.clone())
                                                .when(col.is_primary_key, |el| {
                                                    el.child(
                                                        div()
                                                            .px_1()
                                                            .rounded_sm()
                                                            .bg(rgb(0xf9e2af))
                                                            .text_color(rgb(0x11111b))
                                                            .child("PK"),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_color(rgb(0x6c7086))
                                                .child(type_repr),
                                        ),
                                );
                            }

                            let is_collapsed = self.collapsed_tables.contains(&tbl_name);
                            let tbl_name_for_click = tbl_name.clone();
                            let tbl_name_for_toggle = tbl_name.clone();
                            table_nodes.push(
                                div()
                                    .flex()
                                    .flex_col()
                                    .mb_1()
                                    .child(
                                        div()
                                            .id(ElementId::Name(format!("tbl-node-{}", tbl_name).into()))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .when(is_selected, |el| el.bg(rgb(0x313244)))
                                            .hover(|s| s.bg(rgb(0x45475a)))
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                let t_name = tbl_name_for_click.clone();
                                                move |_this, _event, _window, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.select_table(t_name.clone(), cx);
                                                    });
                                                }
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .id(ElementId::Name(format!("toggle-collapse-{}", tbl_name).into()))
                                                            .text_xs()
                                                            .text_color(rgb(0x6c7086))
                                                            .child(if is_collapsed { "▶" } else { "▼" })
                                                            .on_click(cx.listener({
                                                                let t_name = tbl_name_for_toggle.clone();
                                                                move |this, _event, _window, cx| {
                                                                    this.toggle_table_collapse(&t_name);
                                                                    cx.notify();
                                                                }
                                                            }))
                                                    )
                                                    .child("📊")
                                                    .child(
                                                        div()
                                                            .font_weight(
                                                                gpui::FontWeight::SEMIBOLD,
                                                            )
                                                            .text_xs()
                                                            .text_color(rgb(0xcdd6f4))
                                                            .child(tbl_name),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x6c7086))
                                                    .child(format!("{} cols", tbl.columns.len())),
                                            ),
                                    )
                                    .when(!is_collapsed, |el| el.children(col_nodes)),
                            );
                        }
                        div().flex().flex_col().children(table_nodes)
                    }),
            )
    }
}
