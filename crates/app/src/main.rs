mod state;
mod views;

use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Entity,
    ExternalPaths, KeyBinding, Window, WindowBounds, WindowOptions,
};
use state::{ActiveTab, AppState};
use views::{DiffView, QueryView, SchemaTreeView, TableView};

actions!(strata, [ExecuteQuery, SwitchToQueryTab, SwitchToDiffTab]);

pub struct MainView {
    state: Entity<AppState>,
    schema_tree: Entity<SchemaTreeView>,
    query_view: Entity<QueryView>,
    table_view: Entity<TableView>,
    diff_view: Entity<DiffView>,
}

impl MainView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_cx| AppState::new());
        let schema_tree = cx.new(|_cx| SchemaTreeView::new(state.clone()));
        let query_view = cx.new(|_cx| QueryView::new(state.clone()));
        let table_view = cx.new(|_cx| TableView::new(state.clone()));
        let diff_view = cx.new(|_cx| DiffView::new(state.clone()));

        // Observe state changes to trigger re-rendering
        cx.observe(&state, |_this, _entity, cx| {
            cx.notify();
        })
        .detach();

        Self {
            state,
            schema_tree,
            query_view,
            table_view,
            diff_view,
        }
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let active_tab = app_state.active_tab;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x11111b))
            .text_color(rgb(0xcdd6f4))
            .font_family(".SystemUIFont")
            // Keybinding handler for Cmd/Ctrl + Enter
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ExecuteQuery, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.execute_query(cx);
                    });
                }
            }))
            // Window drag-and-drop file receiver
            .on_drop::<ExternalPaths>(cx.listener({
                let state_model = self.state.clone();
                move |_this, paths: &ExternalPaths, _window, cx| {
                    for path in paths.paths() {
                        let path_str = path.to_string_lossy().to_string();
                        if path_str.ends_with(".sqlite")
                            || path_str.ends_with(".sqlite3")
                            || path_str.ends_with(".db")
                            || path_str.ends_with(".duckdb")
                            || path_str.ends_with(".ddb")
                            || path_str.ends_with(".parquet")
                            || path_str.ends_with(".pq")
                        {
                            state_model.update(cx, |state, cx| {
                                state.load_file(path_str, cx);
                            });
                            break;
                        }
                    }
                }
            }))
            .child(
                // Top Global App Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x181825))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                            .text_base()
                                            .text_color(rgb(0xcba6f7))
                                            .child("⛰️ STRATA"),
                                    ),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x313244))
                                    .text_xs()
                                    .text_color(rgb(0x89b4fa))
                                    .child("GPU Native • Metal/Vulkan"),
                            ),
                    )
                    .child(
                        // Tab Selector
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .p_0p5()
                            .rounded_lg()
                            .bg(rgb(0x11111b))
                            .child(
                                div()
                                    .id("tab-query-btn")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(active_tab == ActiveTab::QueryAndData, |el| {
                                        el.bg(rgb(0x313244)).font_weight(gpui::FontWeight::BOLD)
                                    })
                                    .text_xs()
                                    .text_color(rgb(0xcdd6f4))
                                    .child("Data Grid & SQL")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.set_active_tab(ActiveTab::QueryAndData, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("tab-diff-btn")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(active_tab == ActiveTab::SchemaDiff, |el| {
                                        el.bg(rgb(0x313244)).font_weight(gpui::FontWeight::BOLD)
                                    })
                                    .text_xs()
                                    .text_color(rgb(0xcdd6f4))
                                    .child("Schema Diff & Migrations")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _event, _window, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.set_active_tab(ActiveTab::SchemaDiff, cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .child(
                        // Metrics & Engine badges
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x1e1e2e))
                                    .text_xs()
                                    .text_color(rgb(0xa6e3a1))
                                    .child("<60MB RAM"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x1e1e2e))
                                    .text_xs()
                                    .text_color(rgb(0xf9e2af))
                                    .child("<50ms Boot"),
                            ),
                    ),
            )
            .child(
                // Body Layout (Sidebar + Main View)
                div()
                    .flex_1()
                    .flex()
                    .child(self.schema_tree.clone())
                    .child(if active_tab == ActiveTab::QueryAndData {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(self.query_view.clone())
                            .child(self.table_view.clone())
                    } else {
                        div().flex_1().flex().child(self.diff_view.clone())
                    }),
            )
    }
}

fn main() {
    env_logger::init();

    Application::new().run(|cx: &mut App| {
        // Register keybindings
        cx.bind_keys([
            KeyBinding::new("ctrl-enter", ExecuteQuery, None),
            KeyBinding::new("cmd-enter", ExecuteQuery, None),
            KeyBinding::new("ctrl-1", SwitchToQueryTab, None),
            KeyBinding::new("ctrl-2", SwitchToDiffTab, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Strata - GPU Accelerated Local Database Client".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::Point::new(px(12.0), px(12.0))),
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| MainView::new(cx)),
        )
        .expect("Failed to initialize Strata GPUI desktop window");
    });
}
