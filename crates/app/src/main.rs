#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod state;
mod views;

use gpui::{
    actions, div, img, prelude::*, px, rgb, size, App, Application, AssetSource, Bounds, Context,
    Entity, ExternalPaths, KeyBinding, MouseButton, SharedString, Window, WindowBounds,
    WindowControlArea, WindowDecorations, WindowOptions,
};
use state::{ActiveTab, AppState, UpdateState};
use views::{DiffView, QueryView, SchemaTreeView, TableView};

struct StrataAssets;

impl AssetSource for StrataAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match path {
            "icons/dark-mode.svg" | "icons/logo-dark.svg" | "dark-mode.svg" => {
                Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                    "../../../assets/dark-mode.svg"
                ))))
            }
            "icons/light-mode.svg" | "icons/logo-light.svg" | "light-mode.svg" => {
                Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                    "../../../assets/light-mode.svg"
                ))))
            }
            "icons/icon.png" | "icon.png" => Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../../../assets/icon.png"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(vec![
            "icons/dark-mode.svg".into(),
            "icons/light-mode.svg".into(),
            "icons/icon.png".into(),
        ])
    }
}

actions!(
    strata,
    [
        ExecuteQuery,
        SwitchToQueryTab,
        SwitchToDiffTab,
        OpenFileDialog,
        CheckForUpdatesAction
    ]
);

pub struct MainView {
    state: Entity<AppState>,
    schema_tree: Entity<SchemaTreeView>,
    query_view: Entity<QueryView>,
    table_view: Entity<TableView>,
    diff_view: Entity<DiffView>,
}

impl MainView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| AppState::new(cx));
        let schema_tree = cx.new(|_cx| SchemaTreeView::new(state.clone()));
        let query_view = cx.new(|cx| QueryView::new(state.clone(), cx));
        let table_view = cx.new(|_cx| TableView::new(state.clone()));
        let diff_view = cx.new(|_cx| DiffView::new(state.clone()));

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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let active_tab = app_state.active_tab;
        let update_state = app_state.update_state.clone();
        let current_version = app_state.current_version.clone();
        let is_maximized = window.is_maximized();

        let update_badge = match update_state {
            UpdateState::Available { ref version, .. } => div()
                .id("top-update-install-btn")
                .px_2p5()
                .py_1()
                .rounded(px(5.0))
                .bg(rgb(0x112820))
                .border_1()
                .border_color(rgb(0x1f4d3b))
                .hover(|s| s.bg(rgb(0x17382c)))
                .cursor_pointer()
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(rgb(0x3dd68c))
                .child(format!("Update v{}", version))
                .on_click(cx.listener({
                    let state_model = self.state.clone();
                    move |_this, _ev, _win, cx| {
                        state_model.update(cx, |state, cx| {
                            state.apply_update_now(cx);
                        });
                    }
                })),
            UpdateState::Downloading(ref version) => div()
                .id("top-update-downloading")
                .px_2p5()
                .py_1()
                .rounded(px(5.0))
                .bg(rgb(0x141720))
                .text_xs()
                .text_color(rgb(0xf5a623))
                .child(format!("Updating v{}...", version)),
            UpdateState::ReadyToRestart(ref version) => div()
                .id("top-update-ready")
                .px_2p5()
                .py_1()
                .rounded(px(5.0))
                .bg(rgb(0x112820))
                .text_xs()
                .text_color(rgb(0x3dd68c))
                .child(format!("v{} Installed", version)),
            UpdateState::Checking => div()
                .id("top-update-checking")
                .px_2p5()
                .py_1()
                .rounded(px(5.0))
                .bg(rgb(0x141720))
                .text_xs()
                .text_color(rgb(0x949cbb))
                .child("Checking updates..."),
            _ => div()
                .id("top-update-check-btn")
                .px_2p5()
                .py_1()
                .rounded(px(5.0))
                .bg(rgb(0x141720))
                .hover(|s| s.bg(rgb(0x1c202c)))
                .cursor_pointer()
                .text_xs()
                .text_color(rgb(0x949cbb))
                .child(format!("v{} • Check Updates", current_version))
                .on_click(cx.listener({
                    let state_model = self.state.clone();
                    move |_this, _ev, _win, cx| {
                        state_model.update(cx, |state, cx| {
                            state.check_updates_now(cx);
                        });
                    }
                })),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x090a0f))
            .text_color(rgb(0xeceff4))
            .font_family(".SystemUIFont")
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ExecuteQuery, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.execute_query(cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &SwitchToQueryTab, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.set_active_tab(ActiveTab::QueryAndData, cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &SwitchToDiffTab, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.set_active_tab(ActiveTab::SchemaDiff, cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &OpenFileDialog, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.prompt_open_file(cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &CheckForUpdatesAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.check_updates_now(cx);
                    });
                }
            }))
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
                // Integrated Upper Taskbar (Custom Titlebar with Window Controls, No Brand Title Text, No Clutter)
                div()
                    .id("custom-top-taskbar")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(38.0))
                    .pl_3()
                    .bg(rgb(0x0f1117))
                    .border_b_1()
                    .border_color(rgb(0x1e2230))
                    .window_control_area(WindowControlArea::Drag)
                    .when(cfg!(not(target_os = "windows")), |el| {
                        el.on_mouse_down(MouseButton::Left, |_, window, _| {
                            window.start_window_move();
                        })
                    })
                    .child(
                        // Left: Subtle Icon (no "Strata" name text) + Segmented View Switcher
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                img("icons/dark-mode.svg")
                                    .size(px(18.0))
                                    .flex_shrink_0(),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .p_0p5()
                                    .rounded(px(6.0))
                                    .bg(rgb(0x090a0f))
                                    .border_1()
                                    .border_color(rgb(0x181c26))
                                    .child(
                                        div()
                                            .id("tab-query-btn")
                                            .px_3()
                                            .py_1()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .when(active_tab == ActiveTab::QueryAndData, |el| {
                                                el.bg(rgb(0x1c202c))
                                                    .text_color(rgb(0xffffff))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                            })
                                            .when(active_tab != ActiveTab::QueryAndData, |el| {
                                                el.text_color(rgb(0x949cbb))
                                            })
                                            .text_xs()
                                            .child("Workspace")
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                move |_this, _event, _window, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.set_active_tab(
                                                            ActiveTab::QueryAndData,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("tab-diff-btn")
                                            .px_3()
                                            .py_1()
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .when(active_tab == ActiveTab::SchemaDiff, |el| {
                                                el.bg(rgb(0x1c202c))
                                                    .text_color(rgb(0xffffff))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                            })
                                            .when(active_tab != ActiveTab::SchemaDiff, |el| {
                                                el.text_color(rgb(0x949cbb))
                                            })
                                            .text_xs()
                                            .child("Schema Diff")
                                            .on_click(cx.listener({
                                                let state_model = self.state.clone();
                                                move |_this, _event, _window, cx| {
                                                    state_model.update(cx, |state, cx| {
                                                        state.set_active_tab(
                                                            ActiveTab::SchemaDiff,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        // Right: Updater Action + Native-Behaving Window Control Buttons (Minimize, Maximize, Close)
                        div()
                            .flex()
                            .items_center()
                            .h_full()
                            .gap_2()
                            .child(update_badge)
                            .child(
                                div()
                                    .w(px(1.0))
                                    .h(px(16.0))
                                    .bg(rgb(0x1e2230)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .h_full()
                                    // Minimize Button
                                    .child(
                                        div()
                                            .id("window-btn-minimize")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(44.0))
                                            .h_full()
                                            .hover(|s| s.bg(rgb(0x1c202c)))
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Min)
                                            .when(cfg!(not(target_os = "windows")), |el| {
                                                el.on_click(|_, window, _| {
                                                    window.minimize_window();
                                                })
                                            })
                                            .text_xs()
                                            .text_color(rgb(0xc8d0e0))
                                            .child("─"),
                                    )
                                    // Maximize / Restore Button
                                    .child(
                                        div()
                                            .id("window-btn-maximize")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(44.0))
                                            .h_full()
                                            .hover(|s| s.bg(rgb(0x1c202c)))
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Max)
                                            .when(cfg!(not(target_os = "windows")), |el| {
                                                el.on_click(|_, window, _| {
                                                    window.zoom_window();
                                                })
                                            })
                                            .text_xs()
                                            .text_color(rgb(0xc8d0e0))
                                            .child(if is_maximized { "❐" } else { "□" }),
                                    )
                                    // Close Button
                                    .child(
                                        div()
                                            .id("window-btn-close")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(46.0))
                                            .h_full()
                                            .hover(|s| s.bg(rgb(0xe5484d)).text_color(rgb(0xffffff)))
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Close)
                                            .when(cfg!(not(target_os = "windows")), |el| {
                                                el.on_click(|_, window, cx| {
                                                    window.remove_window();
                                                    cx.quit();
                                                })
                                            })
                                            .text_xs()
                                            .text_color(rgb(0xc8d0e0))
                                            .child("✕"),
                                    ),
                            ),
                    ),
            )
            .child(
                // Main Workspace Body (Sidebar + Active Panel)
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(self.schema_tree.clone())
                    .child(if active_tab == ActiveTab::QueryAndData {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(self.query_view.clone())
                            .child(self.table_view.clone())
                    } else {
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            .child(self.diff_view.clone())
                    }),
            )
    }
}

fn main() {
    env_logger::init();

    Application::new()
        .with_assets(StrataAssets)
        .run(|cx: &mut App| {
            cx.bind_keys([
                KeyBinding::new("ctrl-enter", ExecuteQuery, None),
                KeyBinding::new("cmd-enter", ExecuteQuery, None),
                KeyBinding::new("ctrl-1", SwitchToQueryTab, None),
                KeyBinding::new("cmd-1", SwitchToQueryTab, None),
                KeyBinding::new("ctrl-2", SwitchToDiffTab, None),
                KeyBinding::new("cmd-2", SwitchToDiffTab, None),
                KeyBinding::new("ctrl-o", OpenFileDialog, None),
                KeyBinding::new("cmd-o", OpenFileDialog, None),
                KeyBinding::new("ctrl-u", CheckForUpdatesAction, None),
            ]);

            let bounds = Bounds::centered(None, size(px(1240.0), px(800.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(860.0), px(540.0))),
                    window_decorations: Some(WindowDecorations::Client),
                    app_id: Some("tech.soventa.strata".into()),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Strata".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(gpui::Point::new(px(12.0), px(12.0))),
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| MainView::new(cx)),
            )
            .expect("Failed to initialize Strata desktop window");
        });
}
