use crate::state::AppState;
use crate::theme::ThemePalette;
use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

pub struct QueryView {
    pub state: Entity<AppState>,
    pub focus_handle: FocusHandle,
}

impl QueryView {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            focus_handle: cx.focus_handle(),
        }
    }

    fn is_sql_keyword(word: &str) -> bool {
        let upper = word
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .to_ascii_uppercase();
        matches!(
            upper.as_str(),
            "SELECT"
                | "FROM"
                | "WHERE"
                | "GROUP"
                | "BY"
                | "ORDER"
                | "ASC"
                | "DESC"
                | "LIMIT"
                | "OFFSET"
                | "HAVING"
                | "JOIN"
                | "LEFT"
                | "INNER"
                | "RIGHT"
                | "OUTER"
                | "ON"
                | "AS"
                | "AND"
                | "OR"
                | "NOT"
                | "IN"
                | "IS"
                | "NULL"
                | "COUNT"
                | "SUM"
                | "AVG"
                | "MIN"
                | "MAX"
                | "DISTINCT"
                | "INSERT"
                | "INTO"
                | "VALUES"
                | "UPDATE"
                | "SET"
                | "DELETE"
                | "CREATE"
                | "TABLE"
                | "ALTER"
                | "DROP"
                | "VIEW"
                | "INDEX"
                | "PRIMARY"
                | "KEY"
                | "FOREIGN"
                | "REFERENCES"
                | "PRAGMA"
                | "DESCRIBE"
                | "SHOW"
                | "SUMMARIZE"
                | "WITH"
                | "CASE"
                | "WHEN"
                | "THEN"
                | "ELSE"
                | "END"
                | "UNION"
                | "ALL"
        )
    }

    fn render_highlighted_line(line: &str, theme: &ThemePalette) -> Div {
        let trimmed = line.trim_start();
        if trimmed.starts_with("--") {
            return div()
                .flex_1()
                .text_xs()
                .font_family("monospace")
                .text_color(theme.text_muted_rgb())
                .child(line.to_string());
        }

        // Tokenize preserving spaces so syntax colors align naturally in a flex row
        let mut spans = Vec::new();
        for token in line.split_inclusive(' ') {
            let trimmed_tok = token.trim();
            let is_str = trimmed_tok.starts_with('\'') || trimmed_tok.ends_with('\'');
            let is_num = !trimmed_tok.is_empty()
                && trimmed_tok
                    .trim_end_matches(|c: char| c == ';' || c == ',' || c == ')')
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.');
            let is_kw = Self::is_sql_keyword(trimmed_tok);

            let (color, bold) = if is_str {
                (theme.accent_rgb(), false)
            } else if is_kw {
                (theme.telemetry_rgb(), true)
            } else if is_num {
                (theme.telemetry_rgb(), false)
            } else {
                (theme.text_primary_rgb(), false)
            };

            spans.push(
                div()
                    .text_xs()
                    .font_family("monospace")
                    .text_color(color)
                    .when(bold, |el| el.font_weight(gpui::FontWeight::SEMIBOLD))
                    .child(token.to_string()),
            );
        }

        if spans.is_empty() {
            div()
                .flex_1()
                .text_xs()
                .font_family("monospace")
                .text_color(theme.text_primary_rgb())
                .child(String::new())
        } else {
            div().flex_1().flex().flex_wrap().children(spans)
        }
    }
}

impl Render for QueryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let query_buffer = app_state.query_buffer.clone();
        let query_cursor = app_state.query_cursor;
        let has_history = !app_state.query_history.is_empty();
        let is_executing = app_state.is_executing;
        let exec_time = app_state.execution_time_ms;
        let error_message = app_state.error_message.clone();
        let default_row_limit = app_state.default_row_limit;
        let selected_table = app_state.selected_table.clone();
        let is_focused = self.focus_handle.is_focused(window);

        // Insert visible cursor marker `▏` at `query_cursor` when focused
        let display_buffer = if is_focused {
            let char_count = query_buffer.chars().count();
            let pos = query_cursor.min(char_count);
            let byte_idx = query_buffer
                .char_indices()
                .nth(pos)
                .map(|(i, _)| i)
                .unwrap_or(query_buffer.len());
            let mut buf = query_buffer.clone();
            buf.insert(byte_idx, '▏');
            buf
        } else {
            query_buffer.clone()
        };

        let lines: Vec<String> = if display_buffer.is_empty() {
            vec![String::new()]
        } else {
            display_buffer.split('\n').map(|s| s.to_string()).collect()
        };

        let mut line_rows = Vec::new();
        for (idx, line_text) in lines.iter().enumerate() {
            line_rows.push(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_0p5()
                    .child(
                        div()
                            .w(px(22.0))
                            .text_right()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(theme.text_muted_rgb())
                            .child(format!("{}", idx + 1)),
                    )
                    .child(Self::render_highlighted_line(&line_text, &theme)),
            );
        }

        let table_for_select = selected_table.clone();
        let table_for_count = selected_table.clone();

        let chip_bg = theme.surface_container_rgb();
        let chip_hover = theme.surface_container_high_rgb();
        let chip_fg = theme.text_secondary_rgb();

        div()
            .flex()
            .flex_col()
            .bg(theme.surface_rgb())
            .border_b_1()
            .border_color(theme.outline_rgb())
            .child(
                // Material Action Toolbar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3p5()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.outline_variant_rgb())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                // Key CTA Run Button (Soft Dried Apricot #E08A63)
                                div()
                                    .id("run-query-btn")
                                    .flex()
                                    .items_center()
                                    .px_3p5()
                                    .py_1()
                                    .rounded(px(8.0))
                                    .bg(if is_executing {
                                        theme.surface_container_high_rgb()
                                    } else {
                                        theme.accent_rgb()
                                    })
                                    .hover({
                                        let h = theme.accent_hover_rgb();
                                        move |s| s.bg(h)
                                    })
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.on_accent_rgb())
                                    .child(if is_executing { "Running" } else { "Run" })
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.execute_query(cx);
                                            });
                                        }
                                    })),
                            )
                            .when_some(table_for_select, |el, tbl| {
                                el.child(
                                    div()
                                        .id("chip-select-top")
                                        .px_2p5()
                                        .py_1()
                                        .rounded(px(6.0))
                                        .bg(chip_bg)
                                        .hover(move |s| s.bg(chip_hover))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(chip_fg)
                                        .child(format!("Limit {}", default_row_limit))
                                        .on_click(cx.listener({
                                            let state_model = self.state.clone();
                                            move |_this, _event, _window, cx| {
                                                let sql = format!(
                                                    "SELECT * FROM \"{}\" LIMIT {};",
                                                    tbl, default_row_limit
                                                );
                                                state_model.update(cx, |state, cx| {
                                                    state.set_query_and_run(sql, cx);
                                                });
                                            }
                                        })),
                                )
                            })
                            .when_some(table_for_count, |el, tbl| {
                                el.child(
                                    div()
                                        .id("chip-count-rows")
                                        .px_2p5()
                                        .py_1()
                                        .rounded(px(6.0))
                                        .bg(chip_bg)
                                        .hover(move |s| s.bg(chip_hover))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(chip_fg)
                                        .child("Count")
                                        .on_click(cx.listener({
                                            let state_model = self.state.clone();
                                            move |_this, _event, _window, cx| {
                                                let sql = format!(
                                                    "SELECT COUNT(*) AS total_rows FROM \"{}\";",
                                                    tbl
                                                );
                                                state_model.update(cx, |state, cx| {
                                                    state.set_query_and_run(sql, cx);
                                                });
                                            }
                                        })),
                                )
                            })
                            .when(has_history, |el| {
                                el.child(
                                    div()
                                        .id("chip-query-history")
                                        .px_2p5()
                                        .py_1()
                                        .rounded(px(6.0))
                                        .bg(chip_bg)
                                        .hover(move |s| s.bg(chip_hover))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(chip_fg)
                                        .child("History")
                                        .on_click(cx.listener({
                                            let state_model = self.state.clone();
                                            move |_this, _event, _window, cx| {
                                                state_model.update(cx, |state, cx| {
                                                    state.cycle_query_history(cx);
                                                });
                                            }
                                        })),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .when_some(exec_time, |el, ms| {
                                el.child(
                                    div()
                                        .px_2p5()
                                        .py_0p5()
                                        .rounded(px(10.0))
                                        .bg(theme.telemetry_container_rgb())
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.telemetry_rgb())
                                        .child(format!("{} ms", ms)),
                                )
                            })
                            .child(
                                div()
                                    .id("btn-format-sql")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(chip_bg)
                                    .hover(move |s| s.bg(chip_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(chip_fg)
                                    .child("Format")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.format_query_buffer(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-paste-sql")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(chip_bg)
                                    .hover(move |s| s.bg(chip_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(chip_fg)
                                    .child("Paste")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            if let Some(item) = cx.read_from_clipboard() {
                                                if let Some(txt) = item.text() {
                                                    state_model.update(cx, |state, cx| {
                                                        state.set_query_buffer_with_cursor(txt);
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-copy-sql")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(chip_bg)
                                    .hover(move |s| s.bg(chip_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(chip_fg)
                                    .child("Copy")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            let sql = state_model.read(cx).query_buffer.clone();
                                            cx.write_to_clipboard(ClipboardItem::new_string(sql));
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-clear-sql")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(chip_bg)
                                    .hover(move |s| s.bg(chip_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(chip_fg)
                                    .child("Clear")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.clear_query(cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // SQL Editor Surface
                div()
                    .id("sql-editor-surface")
                    .track_focus(&self.focus_handle)
                    .w_full()
                    .min_h(px(72.0))
                    .max_h(px(140.0))
                    .px_3p5()
                    .py_2p5()
                    .bg(theme.surface_lowest_rgb())
                    .border_b_2()
                    .border_color(if is_focused {
                        theme.primary_rgb()
                    } else {
                        theme.surface_lowest_rgb()
                    })
                    .cursor_text()
                    .on_click(cx.listener(|this, _event, window, cx| {
                        window.focus(&this.focus_handle);
                        cx.notify();
                    }))
                    .on_key_down(cx.listener({
                        let state_model = self.state.clone();
                        move |_this, event: &KeyDownEvent, _window, cx| {
                            let key = event.keystroke.key.as_str();
                            let mods = &event.keystroke.modifiers;

                            if (mods.control || mods.platform) && key == "enter" {
                                state_model.update(cx, |state, cx| {
                                    state.execute_query(cx);
                                });
                                return;
                            }

                            if (mods.control || mods.platform) && key.eq_ignore_ascii_case("v") {
                                if let Some(item) = cx.read_from_clipboard() {
                                    if let Some(txt) = item.text() {
                                        state_model.update(cx, |state, cx| {
                                            state.append_to_query(&txt, cx);
                                        });
                                    }
                                }
                                return;
                            }

                            if (mods.control || mods.platform) && key.eq_ignore_ascii_case("c") {
                                let sql = state_model.read(cx).query_buffer.clone();
                                cx.write_to_clipboard(ClipboardItem::new_string(sql));
                                return;
                            }

                            if (mods.control || mods.platform) && key.eq_ignore_ascii_case("l") {
                                state_model.update(cx, |state, cx| {
                                    state.clear_query(cx);
                                });
                                return;
                            }

                            if mods.alt && key == "up" {
                                state_model.update(cx, |state, cx| {
                                    state.cycle_query_history(cx);
                                });
                                return;
                            }

                            match key {
                                "left" => {
                                    state_model.update(cx, |state, cx| {
                                        state.move_cursor_left(cx);
                                    });
                                }
                                "right" => {
                                    state_model.update(cx, |state, cx| {
                                        state.move_cursor_right(cx);
                                    });
                                }
                                "home" => {
                                    state_model.update(cx, |state, cx| {
                                        state.move_cursor_home(cx);
                                    });
                                }
                                "end" => {
                                    state_model.update(cx, |state, cx| {
                                        state.move_cursor_end(cx);
                                    });
                                }
                                "backspace" => {
                                    state_model.update(cx, |state, cx| {
                                        state.backspace_query(cx);
                                    });
                                }
                                "delete" => {
                                    state_model.update(cx, |state, cx| {
                                        state.delete_at_cursor(cx);
                                    });
                                }
                                "enter" => {
                                    state_model.update(cx, |state, cx| {
                                        state.append_to_query("\n", cx);
                                    });
                                }
                                "tab" => {
                                    state_model.update(cx, |state, cx| {
                                        state.append_to_query("  ", cx);
                                    });
                                }
                                "space" => {
                                    state_model.update(cx, |state, cx| {
                                        state.append_to_query(" ", cx);
                                    });
                                }
                                _ => {
                                    if let Some(ref ch) = event.keystroke.key_char {
                                        if !mods.control && !mods.platform && !mods.alt {
                                            state_model.update(cx, |state, cx| {
                                                state.append_to_query(ch, cx);
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
                                            state.append_to_query(&ch, cx);
                                        });
                                    }
                                }
                            }
                        }
                    }))
                    .children(line_rows),
            )
            .when_some(error_message, |el, err| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3p5()
                        .py_1p5()
                        .bg(theme.danger_container_rgb())
                        .border_t_1()
                        .border_color(theme.danger_rgb())
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.danger_rgb())
                                .child(format!("{}", err)),
                        ),
                )
            })
            .with_animation(
                "query-editor-fade",
                Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint()),
                |el, delta| el.opacity(delta),
            )
    }
}
