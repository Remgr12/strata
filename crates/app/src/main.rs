#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod state;
mod theme;
mod views;

use gpui::{
    actions, div, ease_out_quint, prelude::*, px, rgb, size, Animation, AnimationExt, AnyElement,
    App, Application, Bounds, ClipboardItem, Context, ElementId, Entity, ExternalPaths,
    FocusHandle, KeyBinding, KeyDownEvent, MouseButton, Window, WindowBounds, WindowControlArea,
    WindowDecorations, WindowOptions,
};
use state::{ActiveTab, AppState, SettingsSection, TopMenu, UpdateState};
use std::time::Duration;
use strata_core::SqlDialect;
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
        ToggleGeneralSettingsAction,
        ToggleLightDarkAction,
        ToggleSidebarAction,
        ToggleSqlEditorAction
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
        let schema_tree = cx.new(|cx| SchemaTreeView::new(state.clone(), cx));
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

    fn render_top_menu_dropdown(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let app_state = self.state.read(cx);
        let active_menu = app_state.active_top_menu?;
        let theme = app_state.theme.clone();
        let active_tab = app_state.active_tab;
        let show_sidebar = app_state.show_sidebar;
        let show_sql_editor = app_state.show_sql_editor;
        let compact_grid = app_state.compact_grid;
        let default_row_limit = app_state.default_row_limit;
        let auto_query = app_state.auto_query_on_select;
        let has_conn = app_state.file_path.is_some();

        let item_hover = theme.surface_container_high_rgb();
        let text_fg = theme.text_primary_rgb();
        let muted_fg = theme.text_muted_rgb();
        let telemetry_fg = theme.telemetry_rgb();
        let telemetry_bg = theme.telemetry_container_rgb();

        let (left_offset, menu_anim_id, menu_content) = match active_menu {
            TopMenu::File => {
                let content = div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .id("menu-file-open")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(div().text_xs().text_color(text_fg).child("Open File..."))
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(muted_fg)
                                    .child("Ctrl+O"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_open_file(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-file-sqlite")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("New In-Memory SQLite"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(telemetry_fg)
                                    .child(":memory:"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.load_file(":memory:".to_string(), cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-file-duckdb")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("New Scratch DuckDB"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(telemetry_fg)
                                    .child("DuckDB"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.load_file("scratch.duckdb".to_string(), cx);
                                    });
                                }
                            })),
                    )
                    .child(div().my_1().h(px(1.0)).bg(theme.outline_variant_rgb()))
                    .child(
                        div()
                            .id("menu-file-export-csv")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Export Results to CSV..."),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_export_csv(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-file-export-json")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Export Results to JSON..."),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_export_json(cx);
                                    });
                                }
                            })),
                    )
                    .child(div().my_1().h(px(1.0)).bg(theme.outline_variant_rgb()))
                    .child(
                        div()
                            .id("menu-file-snapshot")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Capture Schema Snapshot"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.take_schema_snapshot(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-file-compare")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Compare Baseline File..."),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_compare_file(cx);
                                    });
                                }
                            })),
                    )
                    .when(has_conn, |el| {
                        el.child(
                            div()
                                .id("menu-file-close-conn")
                                .flex()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_1p5()
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .hover(move |s| s.bg(item_hover))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(text_fg)
                                        .child("Close Connection"),
                                )
                                .on_click(cx.listener({
                                    let state_model = self.state.clone();
                                    move |_this, _ev, _win, cx| {
                                        state_model.update(cx, |state, cx| {
                                            state.close_connection(cx);
                                        });
                                    }
                                })),
                        )
                    })
                    .child(div().my_1().h(px(1.0)).bg(theme.outline_variant_rgb()))
                    .child(
                        div()
                            .id("menu-file-quit")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.danger_rgb())
                                    .child("Quit"),
                            )
                            .on_click(|_ev, window, cx| {
                                window.remove_window();
                                cx.quit();
                            }),
                    );
                (px(10.0), "dropdown-menu-file", content)
            }
            TopMenu::View => {
                let content = div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .id("menu-view-workspace")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if active_tab == ActiveTab::QueryAndData {
                                theme.primary_container_rgb()
                            } else {
                                theme.surface_rgb()
                            })
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_fg)
                                            .child("Workspace"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(telemetry_fg)
                                    .child("Ctrl+1"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.set_active_tab(ActiveTab::QueryAndData, cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-view-diff")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if active_tab == ActiveTab::SchemaDiff {
                                theme.primary_container_rgb()
                            } else {
                                theme.surface_rgb()
                            })
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_fg)
                                    .child("Schema Diff"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(telemetry_fg)
                                    .child("Ctrl+2"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.set_active_tab(ActiveTab::SchemaDiff, cx);
                                    });
                                }
                            })),
                    )
                    .child(div().my_1().h(px(1.0)).bg(theme.outline_variant_rgb()))
                    .child(
                        div()
                            .id("menu-view-toggle-sidebar")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Schema Sidebar"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(telemetry_bg)
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child(if show_sidebar { "On" } else { "Off" }),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.toggle_sidebar(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-view-toggle-editor")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("SQL Editor Pane"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(telemetry_bg)
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child(if show_sql_editor { "On" } else { "Off" }),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.toggle_sql_editor(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-view-compact-grid")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Compact Data Grid"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(telemetry_bg)
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child(if compact_grid { "On" } else { "Off" }),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.toggle_compact_grid(cx);
                                    });
                                }
                            })),
                    );
                (px(62.0), "dropdown-menu-view", content)
            }
            TopMenu::Settings => {
                let content = div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .id("menu-settings-dark")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if theme.is_dark {
                                theme.primary_container_rgb()
                            } else {
                                theme.surface_rgb()
                            })
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Theme: Aerostat Dark"),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child("#1A2229"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.apply_preset_theme(ThemePalette::aerostat_dark(), cx);
                                        state.close_top_menu(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-light")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(if !theme.is_dark {
                                theme.primary_container_rgb()
                            } else {
                                theme.surface_rgb()
                            })
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Theme: Aerostat Light"),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child("#D4DFE6"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.apply_preset_theme(
                                            ThemePalette::aerostat_light(),
                                            cx,
                                        );
                                        state.close_top_menu(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-custom-themes")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.accent_rgb())
                                    .child("Customize Theme Palette..."),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(muted_fg)
                                    .child("Ctrl+T"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.open_settings_drawer(SettingsSection::Themes, cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-import-theme")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Import Theme JSON..."),
                            )
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
                            .id("menu-settings-export-theme")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Export Theme JSON..."),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.prompt_export_theme(cx);
                                    });
                                }
                            })),
                    )
                    .child(div().my_1().h(px(1.0)).bg(theme.outline_variant_rgb()))
                    .child(
                        div()
                            .id("menu-settings-general-panel")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_fg)
                                    .child("General Settings..."),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.open_settings_drawer(SettingsSection::General, cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-row-limit")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Default Row Limit"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(telemetry_bg)
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(telemetry_fg)
                                    .child(format!("{}", default_row_limit)),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.cycle_default_row_limit(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-auto-query")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Auto-Query on Table Select"),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(px(4.0))
                                    .bg(telemetry_bg)
                                    .text_xs()
                                    .text_color(telemetry_fg)
                                    .child(if auto_query { "On" } else { "Off" }),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.toggle_auto_query_on_select(cx);
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .id("menu-settings-check-updates")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(item_hover))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_fg)
                                    .child("Check for Updates"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(muted_fg)
                                    .child("Ctrl+U"),
                            )
                            .on_click(cx.listener({
                                let state_model = self.state.clone();
                                move |_this, _ev, _win, cx| {
                                    state_model.update(cx, |state, cx| {
                                        state.check_updates_now(cx);
                                    });
                                }
                            })),
                    );
                (px(118.0), "dropdown-menu-settings", content)
            }
        };

        Some(
            div()
                .id("top-menu-backdrop")
                .absolute()
                .top(px(40.0))
                .left_0()
                .size_full()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener({
                        let state_model = self.state.clone();
                        move |_this, _ev, _win, cx| {
                            state_model.update(cx, |state, cx| {
                                state.close_top_menu(cx);
                            });
                        }
                    }),
                )
                .child(
                    div()
                        .id("top-menu-popover-card")
                        .absolute()
                        .top(px(4.0))
                        .left(left_offset)
                        .w(px(244.0))
                        .p_1p5()
                        .rounded(px(10.0))
                        .bg(theme.surface_rgb())
                        .border_1()
                        .border_color(theme.outline_rgb())
                        .on_mouse_down(MouseButton::Left, |_ev, _win, cx| {
                            cx.stop_propagation();
                        })
                        .child(menu_content)
                        .with_animation(
                            menu_anim_id,
                            Animation::new(Duration::from_millis(160))
                                .with_easing(ease_out_quint()),
                            |el, delta| el.opacity(delta).mt(px((1.0 - delta) * -6.0)),
                        ),
                )
                .into_any_element(),
        )
    }

    fn render_settings_drawer(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let section = app_state.settings_section;
        let active_field = app_state.editing_theme_field;
        let hex_input = app_state.theme_hex_input.clone();
        let is_hex_focused = self.theme_input_focus.is_focused(window);
        let default_row_limit = app_state.default_row_limit;
        let auto_query = app_state.auto_query_on_select;
        let compact_grid = app_state.compact_grid;
        let show_sidebar = app_state.show_sidebar;
        let show_sql_editor = app_state.show_sql_editor;
        let diff_dialect = app_state.diff_dialect;

        let body_element = if section == SettingsSection::Themes {
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
                        .id(ElementId::Name(format!("theme-token-{:?}", field).into()))
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
                .flex()
                .flex_col()
                .gap_3()
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
                                            state.apply_preset_theme(
                                                ThemePalette::aerostat_dark(),
                                                cx,
                                            );
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
                .child(div().flex().flex_col().gap_1p5().children(token_rows))
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
                                                        state.theme_hex_input =
                                                            txt.trim().to_string();
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
                                                    if !mods.control && !mods.platform && !mods.alt
                                                    {
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
                .into_any_element()
        } else {
            // General Settings Panel
            div()
                .flex()
                .flex_col()
                .gap_2p5()
                .child(
                    div()
                        .id("gen-row-limit")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("Default Row Limit"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .font_family("monospace")
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(format!("{}", default_row_limit)),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.cycle_default_row_limit(cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-auto-query")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("Auto-Query on Table Select"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(if auto_query { "On" } else { "Off" }),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.toggle_auto_query_on_select(cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-compact-grid")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("Compact Grid Density"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(if compact_grid { "On" } else { "Off" }),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.toggle_compact_grid(cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-toggle-sidebar")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("Schema Sidebar"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(if show_sidebar { "On" } else { "Off" }),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.toggle_sidebar(cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-toggle-editor")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("SQL Editor Pane"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(if show_sql_editor { "On" } else { "Off" }),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.toggle_sql_editor(cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-dialect-toggle")
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_2p5()
                        .rounded(px(8.0))
                        .bg(theme.surface_container_rgb())
                        .cursor_pointer()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text_primary_rgb())
                                .child("Migration SQL Dialect"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(6.0))
                                .bg(theme.telemetry_container_rgb())
                                .text_xs()
                                .text_color(theme.telemetry_rgb())
                                .child(match diff_dialect {
                                    SqlDialect::Sqlite => "SQLite",
                                    SqlDialect::DuckDb => "DuckDB",
                                    SqlDialect::Generic => "Generic",
                                }),
                        )
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    let next = match state.diff_dialect {
                                        SqlDialect::Sqlite => SqlDialect::DuckDb,
                                        SqlDialect::DuckDb => SqlDialect::Sqlite,
                                        SqlDialect::Generic => SqlDialect::Sqlite,
                                    };
                                    state.set_diff_dialect(next, cx);
                                });
                            }
                        })),
                )
                .child(
                    div()
                        .id("gen-check-updates-btn")
                        .mt_1()
                        .py_1p5()
                        .px_3()
                        .rounded(px(8.0))
                        .bg(theme.primary_rgb())
                        .cursor_pointer()
                        .text_center()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.on_primary_rgb())
                        .child("Check for Updates")
                        .on_click(cx.listener({
                            let state_model = self.state.clone();
                            move |_this, _ev, _win, cx| {
                                state_model.update(cx, |state, cx| {
                                    state.check_updates_now(cx);
                                });
                            }
                        })),
                )
                .into_any_element()
        };

        div()
            .w(px(292.0))
            .h_full()
            .bg(theme.surface_rgb())
            .border_l_1()
            .border_color(theme.outline_rgb())
            .p_3p5()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                // Drawer Header + Section Switcher (Themes | General)
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .p_0p5()
                            .rounded(px(8.0))
                            .bg(theme.surface_container_rgb())
                            .child(
                                div()
                                    .id("drawer-tab-themes")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(section == SettingsSection::Themes, |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(section != SettingsSection::Themes, |el| {
                                        el.text_color(theme.text_secondary_rgb())
                                    })
                                    .text_xs()
                                    .child("Themes")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.open_settings_drawer(
                                                    SettingsSection::Themes,
                                                    cx,
                                                );
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("drawer-tab-general")
                                    .px_2p5()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(section == SettingsSection::General, |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(section != SettingsSection::General, |el| {
                                        el.text_color(theme.text_secondary_rgb())
                                    })
                                    .text_xs()
                                    .child("General")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.open_settings_drawer(
                                                    SettingsSection::General,
                                                    cx,
                                                );
                                            });
                                        }
                                    })),
                            ),
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
            .child(body_element)
            .with_animation(
                "settings-drawer-slide",
                Animation::new(Duration::from_millis(200)).with_easing(ease_out_quint()),
                |el, delta| el.w(px(292.0 * delta)).opacity(delta),
            )
    }
}

impl Render for MainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = self.state.read(cx);
        let theme = app_state.theme.clone();
        let active_tab = app_state.active_tab;
        let active_top_menu = app_state.active_top_menu;
        let update_state = app_state.update_state.clone();
        let show_theme_editor = app_state.show_theme_editor;
        let show_sidebar = app_state.show_sidebar;
        let show_sql_editor = app_state.show_sql_editor;
        let view_seq = app_state.view_transition_seq;
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
        let menu_hover = theme.surface_container_high_rgb();
        let view_anim_id = ElementId::Name(format!("view-pane-{}", view_seq).into());
        let dropdown_overlay = self.render_top_menu_dropdown(cx);

        div()
            .relative()
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
                        state.open_settings_drawer(SettingsSection::Themes, cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ToggleGeneralSettingsAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.open_settings_drawer(SettingsSection::General, cx);
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
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ToggleSidebarAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.toggle_sidebar(cx);
                    });
                }
            }))
            .on_action(cx.listener({
                let state_model = self.state.clone();
                move |_this, _: &ToggleSqlEditorAction, _window, cx| {
                    state_model.update(cx, |state, cx| {
                        state.toggle_sql_editor(cx);
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
                // Material Design Top App Bar
                // Left: File | View | Settings tabs + active view indicator
                // Center: Dedicated drag region
                // Right: Window Modifiers
                div()
                    .id("custom-top-taskbar")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(40.0))
                    .pl_2p5()
                    .bg(theme.surface_rgb())
                    .border_b_1()
                    .border_color(theme.outline_rgb())
                    .child(
                        // Upper-Left Toolbar: File, View, Settings tabs
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("top-menu-file")
                                    .px_3()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(active_top_menu == Some(TopMenu::File), |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(active_top_menu != Some(TopMenu::File), |el| {
                                        el.text_color(theme.text_primary_rgb())
                                            .hover(move |s| s.bg(menu_hover))
                                    })
                                    .text_xs()
                                    .child("File")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.toggle_top_menu(TopMenu::File, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("top-menu-view")
                                    .px_3()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(active_top_menu == Some(TopMenu::View), |el| {
                                        el.bg(theme.primary_rgb())
                                            .text_color(theme.on_primary_rgb())
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .when(active_top_menu != Some(TopMenu::View), |el| {
                                        el.text_color(theme.text_primary_rgb())
                                            .hover(move |s| s.bg(menu_hover))
                                    })
                                    .text_xs()
                                    .child("View")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.toggle_top_menu(TopMenu::View, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("top-menu-settings")
                                    .px_3()
                                    .py_1()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .when(
                                        active_top_menu == Some(TopMenu::Settings)
                                            || show_theme_editor,
                                        |el| {
                                            el.bg(theme.primary_rgb())
                                                .text_color(theme.on_primary_rgb())
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                        },
                                    )
                                    .when(
                                        active_top_menu != Some(TopMenu::Settings)
                                            && !show_theme_editor,
                                        |el| {
                                            el.text_color(theme.text_primary_rgb())
                                                .hover(move |s| s.bg(menu_hover))
                                        },
                                    )
                                    .text_xs()
                                    .child("Settings")
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                state.toggle_top_menu(TopMenu::Settings, cx);
                                            });
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .mx_1p5()
                                    .w(px(1.0))
                                    .h(px(14.0))
                                    .bg(theme.outline_rgb()),
                            )
                            .child(
                                // Active View Context Pill
                                div()
                                    .id("active-view-pill")
                                    .px_2p5()
                                    .py_0p5()
                                    .rounded(px(10.0))
                                    .bg(theme.telemetry_container_rgb())
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.telemetry_rgb())
                                    .child(match active_tab {
                                        ActiveTab::QueryAndData => "Workspace",
                                        ActiveTab::SchemaDiff => "Schema Diff",
                                    })
                                    .on_click(cx.listener({
                                        let state_model = self.state.clone();
                                        move |_this, _ev, _win, cx| {
                                            state_model.update(cx, |state, cx| {
                                                let next = match state.active_tab {
                                                    ActiveTab::QueryAndData => {
                                                        ActiveTab::SchemaDiff
                                                    }
                                                    ActiveTab::SchemaDiff => {
                                                        ActiveTab::QueryAndData
                                                    }
                                                };
                                                state.set_active_tab(next, cx);
                                            });
                                        }
                                    })),
                            ),
                    )
                    .child(
                        // Center: Dedicated Window Drag Region
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
                        // Right: Window Modifiers (Minimize, Maximize/Restore, Close)
                        div()
                            .flex()
                            .items_center()
                            .h_full()
                            .gap_2()
                            .when_some(update_pill, |el, pill| el.child(pill))
                            .child(
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
                // Main Workspace Body (Animated Sidebar + Animated Active View + Animated Settings/Theme Drawer)
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .when(show_sidebar, |el| el.child(self.schema_tree.clone()))
                    .child(if active_tab == ActiveTab::QueryAndData {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .when(show_sql_editor, |el| el.child(self.query_view.clone()))
                            .child(self.table_view.clone())
                            .with_animation(
                                view_anim_id,
                                Animation::new(Duration::from_millis(200))
                                    .with_easing(ease_out_quint()),
                                |el, delta| el.opacity(0.35 + 0.65 * delta),
                            )
                            .into_any_element()
                    } else {
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            .child(self.diff_view.clone())
                            .with_animation(
                                view_anim_id,
                                Animation::new(Duration::from_millis(200))
                                    .with_easing(ease_out_quint()),
                                |el, delta| el.opacity(0.35 + 0.65 * delta),
                            )
                            .into_any_element()
                    })
                    .when(show_theme_editor, |el| {
                        el.child(self.render_settings_drawer(window, cx))
                    }),
            )
            .when_some(dropdown_overlay, |el, overlay| el.child(overlay))
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
            KeyBinding::new("ctrl-,", ToggleGeneralSettingsAction, None),
            KeyBinding::new("cmd-,", ToggleGeneralSettingsAction, None),
            KeyBinding::new("ctrl-shift-l", ToggleLightDarkAction, None),
            KeyBinding::new("ctrl-b", ToggleSidebarAction, None),
            KeyBinding::new("cmd-b", ToggleSidebarAction, None),
            KeyBinding::new("ctrl-e", ToggleSqlEditorAction, None),
            KeyBinding::new("cmd-e", ToggleSqlEditorAction, None),
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
