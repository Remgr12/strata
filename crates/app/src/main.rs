#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod state;
mod theme;
mod views;

use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, ClipboardItem, Context,
    ElementId, Entity, ExternalPaths, FocusHandle, KeyBinding, KeyDownEvent, MouseButton, Window,
    WindowBounds, WindowControlArea, WindowDecorations, WindowOptions,
};
use state::{ActiveTab, AppState, UpdateState};
use theme::{ThemePalette, ThemeTokenField};
use views::{DiffView, QueryView, SchemaTreeView, TableView};

actions!(
    strata,
    [
        ExecuteQuery,
        SwitchToQueryTab,
        SwitchToDiffTab,
        OpenFileDialog,
        CheckForUpdatesAction,
        ToggleThemeEditorAction,
        ToggleLightDarkAction
    ]
);

pub struct MainView {
    state: Entity<AppState>,
    schema_tree: Entity<SchemaTreeView>,
    query_view: Entity<QueryView>,
    table_view: Entity<TableView>,
    diff_view: Entity<DiffView>,
    theme_input_focus: FocusHandle,
}

impl MainView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| AppState::new(cx));
        let schema_tree = cx.new(|_cx| SchemaTreeView::new(state.clone()));
        let query_view = cx.new(|cx| QueryView::new(state.clone(), cx));
        let table_view = cx.new(|_cx| TableView::new(state.clone()));
        let diff_view = cx.new(|_cx| DiffView::new(state.clone()));
        let theme_input_focus = cx.focus_handle();

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
            theme_input_focus,
        }
    }

    fn render_theme_drawer(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let active_field = app_state.editing_theme_field;
        let hex_input = app_state.theme_hex_input.clone();
        let is_hex_focused = self.theme_input_focus.is_focused(window);

        let fields = [
            (ThemeTokenField::Substrate, theme.substrate),
            (ThemeTokenField::Primary, theme.primary),
            (ThemeTokenField::Accent, theme.accent),
            (ThemeTokenField::Telemetry, theme.telemetry),
            (ThemeTokenField::Surface, theme.surface),
            (ThemeTokenField::TextPrimary, theme.text_primary),
        ];

        let mut token_rows = Vec::new();
        for (field, color_val) in fields {
            let is_selected = active_field == field;
            let hex_label = ThemePalette::format_hex(color_val);
            let row_hover = theme.surface_container_high_rgb();
            let focus_handle = self.theme_input_focus.clone();

            token_rows.push(
                div()
                    .id(ElementId::Name(
                        format!("theme-token-{:?}", field).into(),
                    ))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2p5()
                    .py_2()
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .bg(if is_selected {
                        theme.primary_container_rgb()
                    } else {
                        theme.surface_container_rgb()
                    })
                    .border_1()
                    .border_color(if is_selected {
                        theme.primary_rgb()
                    } else {
                        theme.outline_variant_rgb()
                    })
                    .hover(move |s| s.bg(row_hover))
                    .on_click(cx.listener({
                        let state_model = self.state.clone();
                        move |_this, _ev, window, cx| {
                            window.focus(&focus_handle);
                            state_model.update(cx, |state, cx| {
                                state.select_theme_field(field, cx);
                            });
                        }
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2p5()
                            .child(
                                div()
                                    .size(px(16.0))
                                    .rounded(px(4.0))
                                    .bg(rgb(color_val))
                                    .border_1()
                                    .border_color(theme.outline_rgb()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.text_primary_rgb())
                                    .child(field.label()),
                            ),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(theme.telemetry_rgb())
                            .child(hex_label),
                    ),
            );
        }

        let display_hex = if is_hex_focused {
            format!("{}▏", hex_input)
        } else {
            hex_input.clone()
        };

        div()
            .w(px(288.0))
            .h_full()
            .bg(theme.surface_rgb())
            .border_l_1()
            .border_color(theme.outline_rgb())
            .p_3p5()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary_rgb())
                            .child("THEME"),
                    )
                    .child(
                        div()
                            .id("close-theme-drawer")
                            .px_2()
                            .py_0p5()
                            .rounded(px(6.0))
                            .bg(theme.surface_container_rgb())
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary_rgb())
                            .child("✕")
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.toggle_theme_editor(cx);
                                    });
                                }
                            })),
                    ),
            )
            .child(
                // Built-in Presets (Aerostat Dark #1A2229 & Aerostat Light #D4DFE6)
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("preset-aerostat-dark")
                            .flex_1()
                            .py_1p5()
                            .px_2p5()
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .bg(if theme.is_dark {
                                theme.primary_rgb()
                            } else {
                                theme.surface_container_rgb()
                            })
                            .text_center()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if theme.is_dark {
                                theme.on_primary_rgb()
                            } else {
                                theme.text_primary_rgb()
                            })
                            .child("Dark #1A2229")
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.apply_preset_theme(ThemePalette::aerostat_dark(), cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("preset-aerostat-light")
                            .flex_1()
                            .py_1p5()
                            .px_2p5()
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .bg(if !theme.is_dark {
                                theme.primary_rgb()
                            } else {
                                theme.surface_container_rgb()
                            })
                            .text_center()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if !theme.is_dark {
                                theme.on_primary_rgb()
                            } else {
                                theme.text_primary_rgb()
                            })
                            .child("Light #D4DFE6")
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.apply_preset_theme(
                                            ThemePalette::aerostat_light(),
                                            cx,
                                        );
                                    });
                                }
                            })),
                    ),
            )
            .child(
                // Token list
                div().flex().flex_col().gap_1p5().children(token_rows),
            )
            .child(
                // Hex Editor Box
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .p_2p5()
                    .rounded(px(8.0))
                    .bg(theme.surface_lowest_rgb())
                    .border_1()
                    .border_color(if is_hex_focused {
                        theme.primary_rgb()
                    } else {
                        theme.outline_rgb()
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_muted_rgb())
                                    .child(active_field.label()),
                            )
                            .child(
                                div()
                                    .id("btn-paste-hex")
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(theme.surface_container_rgb())
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(theme.text_secondary_rgb())
                                    .child("Paste")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            if let Some(item) = cx.read_from_clipboard() {
                                                if let Some(txt) = item.text() {
                                                    state_model.update(cx, |state, cx| {
                                                        state.theme_hex_input =
                                                            txt.trim().to_string();
                                                        state.apply_theme_hex_input(cx);
                                                    });
                                                }
                                            }
                                        }
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .id("theme-hex-input-box")
                            .track_focus(&self.theme_input_focus)
                            .flex()
                            .items_center()
                            .justify_between()
                            .py_1()
                            .cursor_text()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                window.focus(&this.theme_input_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, event: &KeyDownEvent, _window, cx| {
                                    let key = event.keystroke.key.as_str();
                                    let mods = &event.keystroke.modifiers;

                                    if (mods.control || mods.platform)
                                        && key.eq_ignore_ascii_case("v")
                                    {
                                        if let Some(item) = cx.read_from_clipboard() {
                                            if let Some(txt) = item.text() {
                                                state_model.update(cx, |state, cx| {
                                                    state.theme_hex_input = txt.trim().to_string();
                                                    state.apply_theme_hex_input(cx);
                                                });
                                            }
                                        }
                                        return;
                                    }

                                    match key {
                                        "backspace" => {
                                            state_model.update(cx, |state, cx| {
                                                state.backspace_theme_hex(cx);
                                            });
                                        }
                                        "enter" => {
                                            state_model.update(cx, |state, cx| {
                                                state.apply_theme_hex_input(cx);
                                            });
                                        }
                                        _ => {
                                            if let Some(ref ch) = event.keystroke.key_char {
                                                if !mods.control && !mods.platform && !mods.alt {
                                                    state_model.update(cx, |state, cx| {
                                                        state.append_theme_hex_char(ch, cx);
                                                    });
                                                }
                                            } else if key.len() == 1
                                                && !mods.control
                                                && !mods.platform
                                                && !mods.alt
                                            {
                                                state_model.update(cx, |state, cx| {
                                                    state.append_theme_hex_char(key, cx);
                                                });
                                            }
                                        }
                                    }
                                }
                            }))
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary_rgb())
                                    .child(display_hex),
                            )
                            .child(
                                div()
                                    .id("btn-apply-hex")
                                    .px_2p5()
                                    .py_0p5()
                                    .rounded(px(6.0))
                                    .bg(theme.accent_rgb())
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.on_accent_rgb())
                                    .child("Apply")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.apply_theme_hex_input(cx);
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // Import / Copy Theme JSON
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("btn-import-theme-json")
                            .flex_1()
                            .py_1p5()
                            .px_2p5()
                            .rounded(px(8.0))
                            .bg(theme.surface_container_rgb())
                            .cursor_pointer()
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_primary_rgb())
                            .child("Import JSON")
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_import_theme(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("btn-copy-theme-json")
                            .flex_1()
                            .py_1p5()
                            .px_2p5()
                            .rounded(px(8.0))
                            .bg(theme.surface_container_rgb())
                            .cursor_pointer()
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_primary_rgb())
                            .child("Copy JSON")
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    let cfg = state_model.read(cx).theme.to_custom_config();
                                    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                                        cx.write_to_clipboard(ClipboardItem::new_string(json));
                                    }
                                }
                            })),
                    ),
            )
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let active_tab = app_state.active_tab;
        let update_state = app_state.update_state.clone();
        let show_theme_editor = app_state.show_theme_editor;
        let is_maximized = window.is_maximized();

        let update_pill = match update_state {
            UpdateState::Available { .. } => Some(
                div()
                    .id("top-update-install-btn")
                    .px_2p5()
                    .py_1()
                    .rounded(px(6.0))
                    .bg(theme.accent_rgb())
                    .cursor_pointer()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.on_accent_rgb())
                    .child("Update")
                    .on_click(cx.listener({
                        let state_model = self.state.clone();
                        move |_this, _ev, _win, cx| {
                            state_model.update(cx, |state, cx| {
                                state.apply_update_now(cx);
                            });
                        }
                    })),
            ),
            UpdateState::Downloading(_) => Some(
                div()
                    .id("top-update-downloading")
                    .px_2p5()
                    .py_1()
                    .rounded(px(6.0))
                    .bg(theme.telemetry_container_rgb())
                    .text_xs()
                    .text_color(theme.telemetry_rgb())
                    .child("Updating..."),
            ),
            _ => None,
        };

        let win_btn_hover = theme.surface_container_high_rgb();
        let win_btn_fg = theme.text_primary_rgb();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.substrate_rgb())
            .text_color(theme.text_primary_rgb())
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
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ToggleThemeEditorAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.toggle_theme_editor(cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ToggleLightDarkAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.toggle_light_dark_theme(cx);
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
                            || path_str.ends_with(".json")
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
                // Material Design Top App Bar:
                // - No upper-left logo
                // - No version counter next to window modifiers
                // - Dedicated middle drag region so Windows hit-testing on tabs and window controls works reliably
                div()
                    .id("custom-top-taskbar")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(40.0))
                    .pl_3()
                    .bg(theme.surface_rgb())
                    .border_b_1()
                    .border_color(theme.outline_rgb())
                    .child(
                        // Left: Material Segmented Navigation Pills (No Logo)
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .p_0p5()
                            .rounded(px(8.0))
                            .bg(theme.surface_container_rgb())
                            .child(
                                div()
                                    .id("tab-query-btn")
                                    .px_3p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(active_tab == ActiveTab::QueryAndData, |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(active_tab != ActiveTab::QueryAndData, |el| {
                                        el.text_color(theme.text_secondary_rgb())
                                    })
                                    .text_xs()
                                    .child("Workspace")
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
                                    .px_3p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(active_tab == ActiveTab::SchemaDiff, |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(active_tab != ActiveTab::SchemaDiff, |el| {
                                        el.text_color(theme.text_secondary_rgb())
                                    })
                                    .text_xs()
                                    .child("Schema Diff")
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
                        // Center: Dedicated Window Drag Region (Sibling to interactive controls)
                        div()
                            .id("titlebar-drag-region")
                            .flex_1()
                            .h_full()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            }),
                    )
                    .child(
                        // Right: Theme Controls & Window Modifiers (No Version Counter)
                        div()
                            .flex()
                            .items_center()
                            .h_full()
                            .gap_2()
                            .when_some(update_pill, |el, pill| el.child(pill))
                            .child(
                                div()
                                    .id("btn-toggle-mode")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(theme.surface_container_rgb())
                                    .hover(move |s| s.bg(win_btn_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(theme.text_secondary_rgb())
                                    .child(if theme.is_dark { "Light" } else { "Dark" })
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.toggle_light_dark_theme(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("btn-open-theme-studio")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .bg(if show_theme_editor {
                                        theme.primary_rgb()
                                    } else {
                                        theme.surface_container_rgb()
                                    })
                                    .hover(move |s| s.bg(win_btn_hover))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(if show_theme_editor {
                                        theme.on_primary_rgb()
                                    } else {
                                        theme.text_secondary_rgb()
                                    })
                                    .child("Theme")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.toggle_theme_editor(cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .w(px(1.0))
                                    .h(px(16.0))
                                    .bg(theme.outline_rgb()),
                            )
                            .child(
                                // Window Modifiers (Minimize, Maximize/Restore, Close)
                                div()
                                    .flex()
                                    .items_center()
                                    .h_full()
                                    .child(
                                        div()
                                            .id("window-btn-minimize")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(44.0))
                                            .h_full()
                                            .hover(move |s| s.bg(win_btn_hover))
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Min)
                                            .on_click(|_, window, cx| {
                                                cx.stop_propagation();
                                                window.minimize_window();
                                            })
                                            .text_xs()
                                            .text_color(win_btn_fg)
                                            .child("─"),
                                    )
                                    .child(
                                        div()
                                            .id("window-btn-maximize")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(44.0))
                                            .h_full()
                                            .hover(move |s| s.bg(win_btn_hover))
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Max)
                                            .on_click(|_, window, cx| {
                                                cx.stop_propagation();
                                                window.zoom_window();
                                            })
                                            .text_xs()
                                            .text_color(win_btn_fg)
                                            .child(if is_maximized { "❐" } else { "□" }),
                                    )
                                    .child(
                                        div()
                                            .id("window-btn-close")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w(px(46.0))
                                            .h_full()
                                            .hover(|s| {
                                                s.bg(rgb(0xD96B6B)).text_color(rgb(0xFFFFFF))
                                            })
                                            .cursor_pointer()
                                            .window_control_area(WindowControlArea::Close)
                                            .on_click(|_, window, cx| {
                                                cx.stop_propagation();
                                                window.remove_window();
                                                cx.quit();
                                            })
                                            .text_xs()
                                            .text_color(win_btn_fg)
                                            .child("✕"),
                                    ),
                            ),
                    ),
            )
            .child(
                // Main Workspace Body (Sidebar + Active Panel + Optional Theme Studio Drawer)
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
                    })
                    .when(show_theme_editor, |el| {
                        el.child(self.render_theme_drawer(window, cx))
                    }),
            )
    }
}

fn main() {
    env_logger::init();

    Application::new().run(|cx: &mut App| {
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
            KeyBinding::new("ctrl-t", ToggleThemeEditorAction, None),
            KeyBinding::new("cmd-t", ToggleThemeEditorAction, None),
            KeyBinding::new("ctrl-shift-l", ToggleLightDarkAction, None),
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
