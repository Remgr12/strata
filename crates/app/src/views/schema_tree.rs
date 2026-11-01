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
        let has_connection = app_state.file_path.is_some();
        let short_name = app_state.short_file_name();
        let full_path = app_state
            .file_path
            .clone()
            .unwrap_or_else(|| "Drop a file or open below".to_string());
        let tables = app_state
            .schema
            .as_ref()
            .map(|s| s.tables.clone())
            .unwrap_or_default();
        let selected_table = app_state.selected_table.clone();
        let is_loading = app_state.is_loading_schema;

        let (engine_fg, engine_bg) = match engine_type.as_str() {
            "SQLite" => (rgb(0x689d6a), rgb(0x14241b)),
            "DuckDB" => (rgb(0xf5a623), rgb(0x281e11)),
            "Parquet" => (rgb(0x5e81f4), rgb(0x131b31)),
            _ => (rgb(0x5c637a), rgb(0x141720)),
        };

        div()
            .flex()
            .flex_col()
            .w(px(264.0))
            .h_full()
            .bg(rgb(0x0f1117))
            .border_r_1()
            .border_color(rgb(0x1e2230))
            .child(
                // Connection Card & Quick Actions
                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2p5()
                    .border_b_1()
                    .border_color(rgb(0x1e2230))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .size(px(7.0))
                                            .rounded_full()
                                            .bg(if has_connection {
                                                rgb(0x3dd68c)
                                            } else {
                                                rgb(0x43495c)
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xeceff4))
                                            .overflow_hidden()
                                            .child(short_name),
                                    ),
                            )
                            .when(has_connection, |el| {
                                el.child(
                                    div()
                                        .px_1p5()
                                        .py_0p5()
                                        .rounded(px(4.0))
                                        .bg(engine_bg)
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(engine_fg)
                                        .child(engine_type),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x5c637a))
                            .overflow_hidden()
                            .child(full_path),
                    )
                    .child(
                        // Action Buttons Row
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .id("sidebar-open-file")
                                    .flex_1()
                                    .py_1()
                                    .px_2()
                                    .rounded(px(5.0))
                                    .bg(rgb(0x5e81f4))
                                    .hover(|s| s.bg(rgb(0x7393f7)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(0xffffff))
                                    .child("Open File")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.prompt_open_file(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("sidebar-demo-db")
                                    .flex_1()
                                    .py_1()
                                    .px_2()
                                    .rounded(px(5.0))
                                    .bg(rgb(0x181c26))
                                    .border_1()
                                    .border_color(rgb(0x252a3a))
                                    .hover(|s| s.bg(rgb(0x212634)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(0xc8d0e0))
                                    .child("Sample DB")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.load_demo_workspace(cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .child(
                        // Secondary quick-connect row for empty scratch engines
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .id("btn-quick-sqlite")
                                    .flex_1()
                                    .py_0p5()
                                    .px_2()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("+ Memory SQLite")
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
                                    .py_0p5()
                                    .px_2()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("+ Scratch DuckDB")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.load_file("scratch.duckdb".to_string(), cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // Section Title Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(0x5c637a))
                            .child("Tables"),
                    )
                    .child(
                        div()
                            .px_1p5()
                            .rounded(px(4.0))
                            .bg(rgb(0x141720))
                            .text_xs()
                            .text_color(rgb(0x949cbb))
                            .child(format!("{}", tables.len())),
                    ),
            )
            .child(
                // Tables Tree Scroll Area
                div()
                    .id("tables-tree-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_3()
                    .child(if is_loading {
                        div()
                            .p_4()
                            .text_center()
                            .text_xs()
                            .text_color(rgb(0x949cbb))
                            .child("Introspecting schema...")
                    } else if tables.is_empty() {
                        div()
                            .p_4()
                            .rounded(px(6.0))
                            .bg(rgb(0x12151e))
                            .border_1()
                            .border_color(rgb(0x1a1e2b))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(0x949cbb))
                                    .child("No tables loaded"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_center()
                                    .text_color(rgb(0x5c637a))
                                    .child("Click 'Sample DB' above or drop a database file into the window."),
                            )
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
                                        .pl_6()
                                        .pr_2()
                                        .py_1()
                                        .rounded(px(4.0))
                                        .hover(|s| s.bg(rgb(0x141720)))
                                        .text_xs()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(
                                                    div()
                                                        .text_color(rgb(0xc8d0e0))
                                                        .child(col.name.clone()),
                                                )
                                                .when(col.is_primary_key, |el| {
                                                    el.child(
                                                        div()
                                                            .px_1()
                                                            .rounded(px(3.0))
                                                            .bg(rgb(0x282012))
                                                            .text_color(rgb(0xf5a623))
                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                            .child("PK"),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .text_color(rgb(0x5c637a))
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
                                    .mb_0p5()
                                    .child(
                                        div()
                                            .id(ElementId::Name(
                                                format!("tbl-node-{}", tbl_name).into(),
                                            ))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px_2()
                                            .py_1p5()
                                            .rounded(px(5.0))
                                            .cursor_pointer()
                                            .when(is_selected, |el| {
                                                el.bg(rgb(0x1c2233))
                                                    .border_1()
                                                    .border_color(rgb(0x2c3652))
                                            })
                                            .hover(|s| s.bg(rgb(0x161a24)))
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
                                                            .id(ElementId::Name(
                                                                format!(
                                                                    "toggle-collapse-{}",
                                                                    tbl_name
                                                                )
                                                                .into(),
                                                            ))
                                                            .w(px(14.0))
                                                            .text_xs()
                                                            .text_color(rgb(0x5c637a))
                                                            .child(if is_collapsed {
                                                                "▸"
                                                            } else {
                                                                "▾"
                                                            })
                                                            .on_click(cx.listener({
                                                                let t_name =
                                                                    tbl_name_for_toggle.clone();
                                                                move |this, _event, _window, cx| {
                                                                    this.toggle_table_collapse(
                                                                        &t_name,
                                                                    );
                                                                    cx.notify();
                                                                }
                                                            })),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_weight(if is_selected {
                                                                gpui::FontWeight::SEMIBOLD
                                                            } else {
                                                                gpui::FontWeight::MEDIUM
                                                            })
                                                            .text_xs()
                                                            .text_color(if is_selected {
                                                                rgb(0xffffff)
                                                            } else {
                                                                rgb(0xeceff4)
                                                            })
                                                            .child(tbl_name),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x5c637a))
                                                    .child(format!("{}", tbl.columns.len())),
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
