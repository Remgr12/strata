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
        let theme = app_state.theme.clone();
        let engine_type = app_state.engine_type.clone();
        let has_connection = app_state.file_path.is_some();
        let short_name = app_state.short_file_name();
        let tables = app_state
            .schema
            .as_ref()
            .map(|s| s.tables.clone())
            .unwrap_or_default();
        let selected_table = app_state.selected_table.clone();
        let is_loading = app_state.is_loading_schema;

        div()
            .flex()
            .flex_col()
            .w(px(256.0))
            .h_full()
            .bg(theme.surface_rgb())
            .border_r_1()
            .border_color(theme.outline_rgb())
            .child(
                // Material Action Header
                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.outline_variant_rgb())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("sidebar-open-file")
                                    .flex_1()
                                    .py_1p5()
                                    .px_3()
                                    .rounded(px(8.0))
                                    .bg(theme.accent_rgb())
                                    .hover({
                                        let h = theme.accent_hover_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.on_accent_rgb())
                                    .child("Open")
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
                                    .py_1p5()
                                    .px_3()
                                    .rounded(px(8.0))
                                    .bg(theme.primary_rgb())
                                    .hover({
                                        let h = theme.primary_hover_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.on_primary_rgb())
                                    .child("Sample")
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
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                div()
                                    .id("btn-quick-sqlite")
                                    .flex_1()
                                    .py_1()
                                    .px_2()
                                    .rounded(px(6.0))
                                    .bg(theme.surface_container_rgb())
                                    .hover({
                                        let h = theme.surface_container_high_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(theme.text_secondary_rgb())
                                    .child("SQLite")
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
                                    .py_1()
                                    .px_2()
                                    .rounded(px(6.0))
                                    .bg(theme.surface_container_rgb())
                                    .hover({
                                        let h = theme.surface_container_high_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_center()
                                    .text_xs()
                                    .text_color(theme.text_secondary_rgb())
                                    .child("DuckDB")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.load_file("scratch.duckdb".to_string(), cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .when(has_connection, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px_2p5()
                                .py_1p5()
                                .rounded(px(8.0))
                                .bg(theme.surface_container_rgb())
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .size(px(6.0))
                                                .rounded_full()
                                                .bg(theme.telemetry_rgb()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(theme.text_primary_rgb())
                                                .overflow_hidden()
                                                .child(short_name),
                                        ),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(4.0))
                                        .bg(theme.telemetry_container_rgb())
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.telemetry_rgb())
                                        .child(engine_type),
                                ),
                        )
                    }),
            )
            .child(
                // Section Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .pt_3()
                    .pb_1p5()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text_muted_rgb())
                            .child("SCHEMA"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded(px(10.0))
                            .bg(theme.telemetry_container_rgb())
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.telemetry_rgb())
                            .child(format!("{}", tables.len())),
                    ),
            )
            .child(
                // Schema Tree Scroll Area
                div()
                    .id("tables-tree-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px_2p5()
                    .pb_3()
                    .child(if is_loading {
                        div()
                            .py_6()
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_muted_rgb())
                            .child("Loading...")
                    } else if tables.is_empty() {
                        div()
                            .py_6()
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_muted_rgb())
                            .child("No tables")
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
                                        .pl_3()
                                        .pr_2()
                                        .py_1()
                                        .rounded(px(6.0))
                                        .text_xs()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(
                                                    div()
                                                        .text_color(theme.text_secondary_rgb())
                                                        .child(col.name.clone()),
                                                )
                                                .when(col.is_primary_key, |el| {
                                                    el.child(
                                                        div()
                                                            .px_1()
                                                            .rounded(px(3.0))
                                                            .bg(theme.accent_container_rgb())
                                                            .text_color(theme.accent_rgb())
                                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                                            .child("PK"),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .text_color(theme.telemetry_rgb())
                                                .child(type_repr),
                                        ),
                                );
                            }

                            let is_collapsed = self.collapsed_tables.contains(&tbl_name);
                            let tbl_name_for_click = tbl_name.clone();
                            let tbl_name_for_toggle = tbl_name.clone();
                            let hover_bg = theme.surface_container_rgb();

                            table_nodes.push(
                                div()
                                    .flex()
                                    .flex_col()
                                    .mb_1()
                                    .child(
                                        div()
                                            .id(ElementId::Name(
                                                format!("tbl-node-{}", tbl_name).into(),
                                            ))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px_2p5()
                                            .py_1p5()
                                            .rounded(px(8.0))
                                            .cursor_pointer()
                                            .when(is_selected, |el| {
                                                el.bg(theme.primary_rgb())
                                            })
                                            .when(!is_selected, |el| {
                                                el.hover(move |s| s.bg(hover_bg))
                                            })
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
                                                            .text_color(if is_selected {
                                                                theme.on_primary_rgb()
                                                            } else {
                                                                theme.text_muted_rgb()
                                                            })
                                                            .child(if is_collapsed {
                                                                "▸"
                                                            } else {
                                                                "▾"
                                                            })
                                                            .on_click(cx.listener({
                                                                let t_name =
                                                                    tbl_name_for_toggle.clone();
                                                                move |this, _event, _window, cx| {
                                                                    cx.stop_propagation();
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
                                                                theme.on_primary_rgb()
                                                            } else {
                                                                theme.text_primary_rgb()
                                                            })
                                                            .child(tbl_name),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(if is_selected {
                                                        theme.on_primary_rgb()
                                                    } else {
                                                        theme.telemetry_rgb()
                                                    })
                                                    .child(format!("{}", tbl.columns.len())),
                                            ),
                                    )
                                    .when(!is_collapsed, |el| {
                                        el.child(
                                            div()
                                                .ml_4()
                                                .mt_0p5()
                                                .border_l_1()
                                                .border_color(theme.outline_variant_rgb())
                                                .flex()
                                                .flex_col()
                                                .children(col_nodes),
                                        )
                                    }),
                            );
                        }
                        div().flex().flex_col().children(table_nodes)
                    }),
            )
    }
}
