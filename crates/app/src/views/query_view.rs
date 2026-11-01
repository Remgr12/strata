use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;

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
}

impl Render for QueryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let query_buffer = app_state.query_buffer.clone();
        let is_executing = app_state.is_executing;
        let exec_time = app_state.execution_time_ms;
        let error_message = app_state.error_message.clone();
        let selected_table = app_state
            .selected_table
            .clone()
            .unwrap_or_else(|| "customers".to_string());
        let is_focused = self.focus_handle.is_focused(window);

        let lines: Vec<String> = if query_buffer.is_empty() {
            vec![String::new()]
        } else {
            query_buffer.lines().map(|s| s.to_string()).collect()
        };

        let mut line_rows = Vec::new();
        for (idx, line_text) in lines.iter().enumerate() {
            let is_last = idx + 1 == lines.len();
            let display_line = if is_last && is_focused {
                format!("{}▏", line_text)
            } else {
                line_text.clone()
            };

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
                            .text_color(rgb(0x43495c))
                            .child(format!("{}", idx + 1)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(rgb(0xeceff4))
                            .child(display_line),
                    ),
            );
        }

        let table_for_select = selected_table.clone();
        let table_for_count = selected_table.clone();

        div()
            .flex()
            .flex_col()
            .bg(rgb(0x0f1117))
            .border_b_1()
            .border_color(rgb(0x1e2230))
            .child(
                // Compact Editor Action Toolbar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0x181c26))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                // Primary Run Query Button
                                div()
                                    .id("run-query-btn")
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .px_3()
                                    .py_1()
                                    .rounded(px(5.0))
                                    .bg(if is_executing {
                                        rgb(0x282d3e)
                                    } else {
                                        rgb(0x5e81f4)
                                    })
                                    .hover(|s| s.bg(rgb(0x7393f7)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(0xffffff))
                                    .child(if is_executing {
                                        "Running..."
                                    } else {
                                        "Run Query"
                                    })
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.execute_query(cx);
                                            });
                                        }
                                    })),
                            )
                            // Quick template chips
                            .child(
                                div()
                                    .id("chip-select-top")
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("Select 100")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            let sql = format!(
                                                "SELECT * FROM \"{}\" LIMIT 100;",
                                                table_for_select
                                            );
                                            state_model.update(cx, |state, cx| {
                                                state.set_query_and_run(sql, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("chip-count-rows")
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("Count Rows")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            let sql = format!(
                                                "SELECT COUNT(*) AS total_rows FROM \"{}\";",
                                                table_for_count
                                            );
                                            state_model.update(cx, |state, cx| {
                                                state.set_query_and_run(sql, cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .child(
                        // Right-hand editor utilities (Paste, Copy, Clear, Exec Time)
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .when_some(exec_time, |el, ms| {
                                el.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(4.0))
                                        .bg(rgb(0x112820))
                                        .text_xs()
                                        .text_color(rgb(0x3dd68c))
                                        .child(format!("{} ms", ms)),
                                )
                            })
                            .child(
                                div()
                                    .id("btn-paste-sql")
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("Paste")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            if let Some(item) = cx.read_from_clipboard() {
                                                if let Some(txt) = item.text() {
                                                    state_model.update(cx, |state, cx| {
                                                        state.query_buffer = txt;
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
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
                                    .child("Copy")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            let sql = state_model.read(cx).query_buffer.clone();
                                            cx.write_to_clipboard(ClipboardItem::new_string(sql));
                                            state_model.update(cx, |state, cx| {
                                                state.status_message =
                                                    Some("Copied SQL query to clipboard".to_string());
                                                cx.notify();
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-clear-sql")
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x141720))
                                    .hover(|s| s.bg(rgb(0x1c202c)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x949cbb))
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
                // Interactive Focusable SQL Code Surface
                div()
                    .id("sql-editor-surface")
                    .track_focus(&self.focus_handle)
                    .w_full()
                    .min_h(px(76.0))
                    .max_h(px(140.0))
                    .px_3()
                    .py_2()
                    .bg(rgb(0x090a0f))
                    .border_b_1()
                    .border_color(if is_focused {
                        rgb(0x5e81f4)
                    } else {
                        rgb(0x141720)
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

                            match key {
                                "backspace" => {
                                    state_model.update(cx, |state, cx| {
                                        state.backspace_query(cx);
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
                        .px_3()
                        .py_1p5()
                        .bg(rgb(0x281218))
                        .border_t_1()
                        .border_color(rgb(0x4d1d29))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xf2777a))
                                .child(format!("{}", err)),
                        ),
                )
            })
    }
}
