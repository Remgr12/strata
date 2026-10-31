use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;

pub struct QueryView {
    pub state: Entity<AppState>,
}

impl QueryView {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for QueryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let query_buffer = app_state.query_buffer.clone();
        let is_executing = app_state.is_executing;
        let exec_time = app_state.execution_time_ms;
        let status_message = app_state.status_message.clone();
        let error_message = app_state.error_message.clone();

        let run_button_label = if is_executing {
            "Executing..."
        } else {
            "▶ Run Query (Ctrl+Enter)"
        };

        div()
            .flex()
            .flex_col()
            .p_3()
            .gap_2()
            .bg(rgb(0x1e1e2e))
            .border_b_1()
            .border_color(rgb(0x313244))
            .child(
                // SQL editor header and action buttons
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x89b4fa))
                                    .child("SQL QUERY BUFFER"),
                            )
                            .child(if let Some(ms) = exec_time {
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x313244))
                                    .text_xs()
                                    .text_color(rgb(0xa6e3a1))
                                    .child(format!("⚡ {}ms", ms))
                            } else {
                                div().text_xs().text_color(rgb(0x6c7086)).child("Ready")
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                // Run Button
                                div()
                                    .id("run-query-btn")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(if is_executing {
                                        rgb(0x585b70)
                                    } else {
                                        rgb(0xa6e3a1)
                                    })
                                    .hover(|s| s.bg(rgb(0x94e2d5)))
                                    .cursor_pointer()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_xs()
                                    .text_color(rgb(0x11111b))
                                    .child(run_button_label)
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.execute_query(cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // SQL Input text display / editor
                div()
                    .w_full()
                    .min_h(px(64.0))
                    .max_h(px(120.0))
                    .p_2()
                    .rounded_md()
                    .bg(rgb(0x11111b))
                    .border_1()
                    .border_color(rgb(0x45475a))
                    .text_xs()
                    .font_family("monospace")
                    .text_color(rgb(0xcdd6f4))
                    .child(query_buffer),
            )
            .child(
                // Status / Error bar
                div().flex().items_center().justify_between().child(
                    if let Some(err) = error_message {
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xf38ba8))
                            .child(format!("❌ {}", err))
                    } else if let Some(status) = status_message {
                        div()
                            .text_xs()
                            .text_color(rgb(0xa6adc8))
                            .child(format!("ℹ️ {}", status))
                    } else {
                        div()
                    },
                ),
            )
    }
}
