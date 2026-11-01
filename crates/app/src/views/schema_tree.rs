use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

pub struct SchemaTreeView {
    pub state: Entity<AppState>,
    pub collapsed_tables: std::collections::HashSet<String>,
    pub filter_focus: FocusHandle,
}

impl SchemaTreeView {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            collapsed_tables: std::collections::HashSet::new(),
            filter_focus: cx.focus_handle(),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let engine_type = app_state.engine_type.clone();
        let has_connection = app_state.file_path.is_some();
        let short_name = app_state.short_file_name();
        let all_tables = app_state
            .schema
            .as_ref()
            .map(|s| s.tables.clone())
            .unwrap_or_default();
        let selected_table = app_state.selected_table.clone();
        let is_loading = app_state.is_loading_schema;
        let schema_filter = app_state.schema_filter.clone();
        let default_row_limit = app_state.default_row_limit;
        let filter_focused = self.filter_focus.is_focused(window);

        let filter_lower = schema_filter.trim().to_ascii_lowercase();
        let tables: Vec<_> = if filter_lower.is_empty() {
            all_tables.clone()
        } else {
            all_tables
                .iter()
                .filter(|t| {
                    t.name.to_ascii_lowercase().contains(&filter_lower)
                        || t.columns
                            .iter()
                            .any(|c| c.name.to_ascii_lowercase().contains(&filter_lower))
                })
                .cloned()
                .collect()
        };

        div()
            .flex()
            .flex_col()
            .w(px(252.0))
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
                            .id("sidebar-open-file")
                            .w_full()
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
                                                .bg(theme.telemetry_rgb())
                                                .with_animation(
                                                    "conn-telemetry-pulse",
                                                    Animation::new(Duration::from_millis(2200))
                                                        .repeat()
                                                        .with_easing(pulsating_between(0.45, 1.0)),
                                                    |dot, delta| dot.opacity(delta),
                                                ),
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
                // Section Header + Filter Box
                div()
                    .flex()
                    .flex_col()
                    .px_3()
                    .pt_2p5()
                    .pb_1p5()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_muted_rgb())
                                    .child("SCHEMA"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .when(has_connection, |el| {
                                        let refresh_hover = theme.surface_container_high_rgb();
                                        el.child(
                                            div()
                                                .id("btn-refresh-schema")
                                                .px_1p5()
                                                .py_0p5()
                                                .rounded(px(6.0))
                                                .bg(theme.surface_container_rgb())
                                                .hover(move |s| s.bg(refresh_hover))
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(theme.text_secondary_rgb())
                                                .child("↻")
                                                .on_click(cx.listener({
                                                    let state_model = self.state.clone();
                                                    move |_this, _ev, _win, cx| {
                                                        state_model.update(cx, |state, cx| {
                                                            state.refresh_schema(cx);
                                                        });
                                                    }
                                                })),
                                        )
                                    })
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
                            ),
                    )
                    .when(!all_tables.is_empty(), |el| {
                        let placeholder_or_val = if schema_filter.is_empty() {
                            if filter_focused {
                                "▏".to_string()
                            } else {
                                "Filter tables…".to_string()
                            }
                        } else if filter_focused {
                            format!("{}▏", schema_filter)
                        } else {
                            schema_filter.clone()
                        };

                        el.child(
                            div()
                                .id("schema-filter-input")
                                .track_focus(&self.filter_focus)
                                .flex()
                                .items_center()
                                .justify_between()
                                .px_2p5()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(theme.surface_lowest_rgb())
                                .border_1()
                                .border_color(if filter_focused {
                                    theme.primary_rgb()
                                } else {
                                    theme.outline_variant_rgb()
                                })
                                .cursor_text()
                                .on_click(cx.listener(|this, _ev, win, cx| {
                                    win.focus(&this.filter_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener({
                                    let state_model = self.state.clone();
                                    move |_this, event: &KeyDownEvent, _win, cx| {
                                        let key = event.keystroke.key.as_str();
                                        let mods = &event.keystroke.modifiers;
                                        match key {
                                            "backspace" => {
                                                state_model.update(cx, |state, cx| {
                                                    state.schema_filter.pop();
                                                    cx.notify();
                                                });
                                            }
                                            "escape" => {
                                                state_model.update(cx, |state, cx| {
                                                    state.schema_filter.clear();
                                                    cx.notify();
                                                });
                                            }
                                            _ => {
                                                if let Some(ref ch) = event.keystroke.key_char {
                                                    if !mods.control && !mods.platform && !mods.alt {
                                                        state_model.update(cx, |state, cx| {
                                                            state.schema_filter.push_str(ch);
                                                            cx.notify();
                                                        });
                                                    }
                                                } else if key.len() == 1
                                                    && !mods.control
                                                    && !mods.platform
                                                    && !mods.alt
                                                {
                                                    let ch = if mods.shift {
                                                        key.to_uppercase()
                                                    } else {
                                                        key.to_string()
                                                    };
                                                    state_model.update(cx, |state, cx| {
                                                        state.schema_filter.push_str(&ch);
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(if schema_filter.is_empty() {
                                            theme.text_muted_rgb()
                                        } else {
                                            theme.text_primary_rgb()
                                        })
                                        .child(placeholder_or_val),
                                )
                                .when(!schema_filter.is_empty(), |box_el| {
                                    box_el.child(
                                        div()
                                            .id("clear-schema-filter")
                                            .cursor_pointer()
                                            .text_xs()
                                            .text_color(theme.text_muted_rgb())
                                            .child("✕")
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                move |_this, _ev, _win, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.schema_filter.clear();
                                                        cx.notify();
                                                    });
                                                }
                                            })),
                                    )
                                }),
                        )
                    }),
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
                            .text_color(theme.telemetry_rgb())
                            .child("Loading...")
                            .with_animation(
                                "schema-loading-pulse",
                                Animation::new(Duration::from_millis(900))
                                    .repeat()
                                    .with_easing(pulsating_between(0.3, 1.0)),
                                |el, delta| el.opacity(delta),
                            )
                            .into_any_element()
                    } else if tables.is_empty() {
                        div()
                            .py_6()
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_muted_rgb())
                            .child("No tables")
                            .into_any_element()
                    } else {
                        let mut table_nodes = Vec::new();
                        for (t_idx, tbl) in tables.iter().enumerate() {
                            let tbl_name = tbl.name.clone();
                            let is_selected = selected_table.as_deref() == Some(&tbl_name);
                            let cols = tbl.columns.clone();

                            let mut col_nodes = Vec::new();
                            for (c_idx, col) in cols.into_iter().enumerate() {
                                let type_repr = col.data_type.sql_repr();
                                let col_name_click = col.name.clone();
                                let tbl_name_click = tbl_name.clone();
                                let col_hover = theme.surface_container_rgb();

                                col_nodes.push(
                                    div()
                                        .id(ElementId::Name(
                                            format!("tree-col-{}-{}", t_idx, c_idx).into(),
                                        ))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .pl_3()
                                        .pr_2()
                                        .py_1()
                                        .rounded(px(6.0))
                                        .hover(move |s| s.bg(col_hover))
                                        .cursor_pointer()
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
                                        )
                                        .on_click(cx.listener({
                                            let state_model = self.state.clone();
                                            move |_this, _ev, _win, cx| {
                                                let escaped_tbl =
                                                    tbl_name_click.replace('"', "\"\"");
                                                let escaped_col =
                                                    col_name_click.replace('"', "\"\"");
                                                let sql = format!(
                                                    "SELECT \"{}\", COUNT(*) AS count FROM \"{}\" GROUP BY \"{}\" ORDER BY count DESC LIMIT {};",
                                                    escaped_col, escaped_tbl, escaped_col, default_row_limit
                                                );
                                                state_model.update(cx, |state, cx| {
                                                    state.selected_table =
                                                        Some(tbl_name_click.clone());
                                                    state.set_query_and_run(sql, cx);
                                                });
                                            }
                                        })),
                                );
                            }

                            let tbl_for_click = tbl_name.clone();
                            let tbl_for_collapse = tbl_name.clone();
                            let is_collapsed = self.collapsed_tables.contains(&tbl_name);
                            let hover_bg = if is_selected {
                                theme.primary_container_rgb()
                            } else {
                                theme.surface_container_rgb()
                            };
                            let expand_anim_id =
                                ElementId::Name(format!("cols-expand-{}", tbl_name).into());

                            table_nodes.push(
                                div()
                                    .flex()
                                    .flex_col()
                                    .mb_1()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .px_2p5()
                                            .py_1p5()
                                            .rounded(px(8.0))
                                            .bg(if is_selected {
                                                theme.primary_container_rgb()
                                            } else {
                                                theme.surface_rgb()
                                            })
                                            .hover(move |s| s.bg(hover_bg))
                                            .cursor_pointer()
                                            .child(
                                                div()
                                                    .id(ElementId::Name(
                                                        format!("tree-tbl-select-{}", t_idx).into(),
                                                    ))
                                                    .flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .flex_1()
                                                    .on_click(cx.listener({
                                                        let state_model = self.state.clone();
                                                        move |_this, _event, _window, cx| {
                                                            let t = tbl_for_click.clone();
                                                            state_model.update(cx, |state, cx| {
                                                                state.select_table(t, cx);
                                                            });
                                                        }
                                                    }))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(if is_selected {
                                                                gpui::FontWeight::SEMIBOLD
                                                            } else {
                                                                gpui::FontWeight::MEDIUM
                                                            })
                                                            .text_color(theme.text_primary_rgb())
                                                            .child(tbl_name.clone()),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .id(ElementId::Name(
                                                        format!("tree-tbl-collapse-{}", t_idx)
                                                            .into(),
                                                    ))
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded(px(4.0))
                                                    .text_xs()
                                                    .text_color(if is_selected {
                                                        theme.text_primary_rgb()
                                                    } else {
                                                        theme.text_muted_rgb()
                                                    })
                                                    .on_click(cx.listener(
                                                        move |this, _event, _window, cx| {
                                                            this.toggle_table_collapse(
                                                                &tbl_for_collapse,
                                                            );
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(if is_collapsed { "+" } else { "−" }),
                                            ),
                                    )
                                    .when(!is_collapsed, |el| {
                                        el.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .mt_0p5()
                                                .ml_2()
                                                .pl_1()
                                                .border_l_1()
                                                .border_color(theme.outline_variant_rgb())
                                                .children(col_nodes)
                                                .with_animation(
                                                    expand_anim_id,
                                                    Animation::new(Duration::from_millis(160))
                                                        .with_easing(ease_out_quint()),
                                                    |cols_el, delta| cols_el.opacity(delta),
                                                ),
                                        )
                                    }),
                            );
                        }
                        div()
                            .flex()
                            .flex_col()
                            .children(table_nodes)
                            .into_any_element()
                    }),
            )
            .with_animation(
                "sidebar-slide-in",
                Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                |el, delta| el.opacity(delta),
            )
    }
}
