use crate::theme::{CustomThemeConfig, ThemePalette, ThemeTokenField};
use crate::views::table_view::format_arrow_cell;
use arrow::record_batch::RecordBatch;
use gpui::prelude::*;
use gpui::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use strata_core::{
    check_for_updates, diff_schemas, download_and_apply_update, DatabaseDriver, DatabaseSchema,
    DuckDbDriver, InstallKind, SchemaDiff, SqlDialect, SqliteDriver, UpdateCheckResult,
    UPDATE_CHECK_INTERVAL_SECS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    QueryAndData,
    SchemaDiff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopMenu {
    File,
    View,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Themes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate(String),
    Available {
        version: String,
        install_kind: InstallKind,
    },
    Downloading(String),
    ReadyToRestart(String),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub show_sidebar: bool,
    pub show_sql_editor: bool,
    pub compact_grid: bool,
    pub default_row_limit: usize,
    pub auto_query_on_select: bool,
    pub recent_files: Vec<String>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            show_sidebar: true,
            show_sql_editor: true,
            compact_grid: false,
            default_row_limit: 100,
            auto_query_on_select: true,
            recent_files: Vec::new(),
        }
    }
}

impl UserPreferences {
    pub fn config_path() -> Option<PathBuf> {
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        }?;
        Some(base.join("strata").join("preferences.json"))
    }

    pub fn load_or_default() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(raw) = std::fs::read_to_string(path) {
                if let Ok(prefs) = serde_json::from_str::<UserPreferences>(&raw) {
                    return prefs;
                }
            }
        }
        Self::default()
    }

    pub fn save_to_disk(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

pub struct AppState {
    pub active_driver: Option<Arc<futures::lock::Mutex<Box<dyn DatabaseDriver>>>>,
    pub file_path: Option<String>,
    pub engine_type: String,
    pub schema: Option<DatabaseSchema>,
    pub query_buffer: String,
    pub query_cursor: usize,
    pub query_history: Vec<String>,
    pub history_index: Option<usize>,
    pub query_results: Option<Vec<RecordBatch>>,
    pub execution_time_ms: Option<u128>,
    pub is_executing: bool,
    pub is_loading_schema: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub active_tab: ActiveTab,
    pub selected_table: Option<String>,
    pub selected_row: Option<usize>,
    pub sort_column: Option<String>,
    pub sort_ascending: bool,

    // Sidebar schema search filter
    pub schema_filter: String,

    // Upper-left toolbar menu & view/general settings
    pub active_top_menu: Option<TopMenu>,
    pub show_sidebar: bool,
    pub show_sql_editor: bool,
    pub compact_grid: bool,
    pub default_row_limit: usize,
    pub auto_query_on_select: bool,
    pub recent_files: Vec<String>,

    // Animation sequence counters
    pub view_transition_seq: usize,
    pub result_seq: usize,

    // Diff view state
    pub snapshot_schema: Option<DatabaseSchema>,
    pub snapshot_label: Option<String>,
    pub schema_diff: Option<SchemaDiff>,
    pub diff_migration_sql: Vec<String>,
    pub diff_dialect: SqlDialect,

    // Auto-updater state
    pub current_version: String,
    pub update_state: UpdateState,
    pub last_update_check: Option<UpdateCheckResult>,

    // Theme & Settings drawer state
    pub theme: ThemePalette,
    pub show_theme_editor: bool,
    pub settings_section: SettingsSection,
    pub editing_theme_field: ThemeTokenField,
    pub theme_hex_input: String,
}

impl AppState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let theme = ThemePalette::load_persisted_or_default();
        let prefs = UserPreferences::load_or_default();
        let initial_hex = theme.get_field_hex(ThemeTokenField::Substrate);

        let mut state = Self {
            active_driver: None,
            file_path: None,
            engine_type: "None".to_string(),
            schema: None,
            query_buffer: String::new(),
            query_cursor: 0,
            query_history: Vec::new(),
            history_index: None,
            query_results: None,
            execution_time_ms: None,
            is_executing: false,
            is_loading_schema: false,
            status_message: None,
            error_message: None,
            active_tab: ActiveTab::QueryAndData,
            selected_table: None,
            selected_row: None,
            sort_column: None,
            sort_ascending: true,
            schema_filter: String::new(),
            active_top_menu: None,
            show_sidebar: prefs.show_sidebar,
            show_sql_editor: prefs.show_sql_editor,
            compact_grid: prefs.compact_grid,
            default_row_limit: prefs.default_row_limit,
            auto_query_on_select: prefs.auto_query_on_select,
            recent_files: prefs.recent_files,
            view_transition_seq: 1,
            result_seq: 1,
            snapshot_schema: None,
            snapshot_label: None,
            schema_diff: None,
            diff_migration_sql: Vec::new(),
            diff_dialect: SqlDialect::Sqlite,
            current_version,
            update_state: UpdateState::Idle,
            last_update_check: None,
            theme,
            show_theme_editor: false,
            settings_section: SettingsSection::Themes,
            editing_theme_field: ThemeTokenField::Substrate,
            theme_hex_input: initial_hex,
        };

        state.start_periodic_update_checker(cx);
        state
    }

    pub fn save_preferences(&self) {
        let prefs = UserPreferences {
            show_sidebar: self.show_sidebar,
            show_sql_editor: self.show_sql_editor,
            compact_grid: self.compact_grid,
            default_row_limit: self.default_row_limit,
            auto_query_on_select: self.auto_query_on_select,
            recent_files: self.recent_files.clone(),
        };
        prefs.save_to_disk();
    }

    pub fn record_recent_file(&mut self, path: &str) {
        if path.is_empty() || path.starts_with(":memory:") {
            return;
        }
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_string());
        if self.recent_files.len() > 6 {
            self.recent_files.truncate(6);
        }
        self.save_preferences();
    }

    pub fn toggle_top_menu(&mut self, menu: TopMenu, cx: &mut Context<Self>) {
        if self.active_top_menu == Some(menu) {
            self.active_top_menu = None;
        } else {
            self.active_top_menu = Some(menu);
        }
        cx.notify();
    }

    pub fn close_top_menu(&mut self, cx: &mut Context<Self>) {
        if self.active_top_menu.is_some() {
            self.active_top_menu = None;
            cx.notify();
        }
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.show_sidebar = !self.show_sidebar;
        self.active_top_menu = None;
        self.save_preferences();
        cx.notify();
    }

    pub fn toggle_sql_editor(&mut self, cx: &mut Context<Self>) {
        self.show_sql_editor = !self.show_sql_editor;
        self.active_top_menu = None;
        self.save_preferences();
        cx.notify();
    }

    pub fn toggle_compact_grid(&mut self, cx: &mut Context<Self>) {
        self.compact_grid = !self.compact_grid;
        self.active_top_menu = None;
        self.save_preferences();
        cx.notify();
    }

    pub fn cycle_default_row_limit(&mut self, cx: &mut Context<Self>) {
        self.default_row_limit = match self.default_row_limit {
            100 => 500,
            500 => 1000,
            _ => 100,
        };
        self.save_preferences();
        cx.notify();
    }

    pub fn toggle_auto_query_on_select(&mut self, cx: &mut Context<Self>) {
        self.auto_query_on_select = !self.auto_query_on_select;
        self.save_preferences();
        cx.notify();
    }

    pub fn open_settings_drawer(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.settings_section = section;
        self.show_theme_editor = true;
        self.active_top_menu = None;
        self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
        cx.notify();
    }

    pub fn close_connection(&mut self, cx: &mut Context<Self>) {
        self.active_driver = None;
        self.file_path = None;
        self.engine_type = "None".to_string();
        self.schema = None;
        self.query_results = None;
        self.selected_table = None;
        self.selected_row = None;
        self.sort_column = None;
        self.execution_time_ms = None;
        self.status_message = None;
        self.error_message = None;
        self.active_top_menu = None;
        cx.notify();
    }

    pub fn toggle_light_dark_theme(&mut self, cx: &mut Context<Self>) {
        self.theme = if self.theme.is_dark {
            ThemePalette::aerostat_light()
        } else {
            ThemePalette::aerostat_dark()
        };
        self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
        let _ = self.theme.save_to_disk();
        cx.notify();
    }

    pub fn apply_preset_theme(&mut self, palette: ThemePalette, cx: &mut Context<Self>) {
        self.theme = palette;
        self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
        let _ = self.theme.save_to_disk();
        cx.notify();
    }

    pub fn toggle_theme_editor(&mut self, cx: &mut Context<Self>) {
        self.show_theme_editor = !self.show_theme_editor;
        self.active_top_menu = None;
        if self.show_theme_editor {
            self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
        }
        cx.notify();
    }

    pub fn select_theme_field(&mut self, field: ThemeTokenField, cx: &mut Context<Self>) {
        self.editing_theme_field = field;
        self.theme_hex_input = self.theme.get_field_hex(field);
        cx.notify();
    }

    pub fn append_theme_hex_char(&mut self, text: &str, cx: &mut Context<Self>) {
        for ch in text.chars() {
            if ch == '#' && self.theme_hex_input.is_empty() {
                self.theme_hex_input.push('#');
            } else if ch.is_ascii_hexdigit() && self.theme_hex_input.len() < 7 {
                if !self.theme_hex_input.starts_with('#') {
                    self.theme_hex_input.insert(0, '#');
                }
                if self.theme_hex_input.len() < 7 {
                    self.theme_hex_input.push(ch.to_ascii_uppercase());
                }
            }
        }
        if self.theme_hex_input.len() == 7 {
            let _ = self
                .theme
                .set_field_hex(self.editing_theme_field, &self.theme_hex_input);
            let _ = self.theme.save_to_disk();
        }
        cx.notify();
    }

    pub fn backspace_theme_hex(&mut self, cx: &mut Context<Self>) {
        self.theme_hex_input.pop();
        cx.notify();
    }

    pub fn apply_theme_hex_input(&mut self, cx: &mut Context<Self>) {
        match self
            .theme
            .set_field_hex(self.editing_theme_field, &self.theme_hex_input)
        {
            Ok(()) => {
                self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
                let _ = self.theme.save_to_disk();
                self.error_message = None;
            }
            Err(err) => {
                self.error_message = Some(err);
            }
        }
        cx.notify();
    }

    pub fn load_theme_from_file(&mut self, path: &str, cx: &mut Context<Self>) {
        match std::fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<CustomThemeConfig>(&raw) {
                Ok(cfg) => match ThemePalette::from_custom_config(&cfg) {
                    Ok(palette) => {
                        self.theme = palette;
                        self.theme_hex_input = self.theme.get_field_hex(self.editing_theme_field);
                        let _ = self.theme.save_to_disk();
                        self.status_message = Some(format!("Theme: {}", self.theme.name));
                    }
                    Err(e) => self.error_message = Some(e),
                },
                Err(e) => self.error_message = Some(format!("Invalid theme JSON: {}", e)),
            },
            Err(e) => self.error_message = Some(format!("Failed to read theme file: {}", e)),
        }
        cx.notify();
    }

    pub fn prompt_import_theme(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import Theme JSON".into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(first) = paths.into_iter().next() {
                    let path_str = first.to_string_lossy().to_string();
                    let _ = this.update(cx, |state, cx| {
                        state.load_theme_from_file(&path_str, cx);
                    });
                }
            }
        })
        .detach();
    }

    pub fn prompt_export_theme(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let receiver = cx.prompt_for_new_path(&dir, Some("strata-theme.json"));
        let json_res = serde_json::to_string_pretty(&self.theme.to_custom_config());

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let _ = this.update(cx, |state, cx| {
                    match &json_res {
                        Ok(json) => match std::fs::write(&path, json) {
                            Ok(()) => {
                                state.status_message =
                                    Some(format!("Exported theme to {}", path.display()));
                            }
                            Err(e) => {
                                state.error_message = Some(format!("Export failed: {}", e));
                            }
                        },
                        Err(e) => {
                            state.error_message = Some(format!("Serialize error: {}", e));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Periodically checks the Forgejo Releases API for newer versions in the background.
    pub fn start_periodic_update_checker(&mut self, cx: &mut Context<Self>) {
        let current_version = self.current_version.clone();
        let background = cx.background_executor().clone();

        cx.spawn(async move |this, cx| {
            background.timer(Duration::from_secs(3)).await;
            loop {
                let ver = current_version.clone();
                let check_res = background
                    .spawn(async move { check_for_updates(&ver) })
                    .await;

                let should_continue = this
                    .update(cx, |state, cx| {
                        match check_res {
                            Ok(result) => {
                                if result.update_available {
                                    state.update_state = UpdateState::Available {
                                        version: result.latest_version.clone(),
                                        install_kind: result.install_kind,
                                    };
                                } else if matches!(
                                    state.update_state,
                                    UpdateState::Idle | UpdateState::Checking
                                ) {
                                    state.update_state =
                                        UpdateState::UpToDate(result.latest_version.clone());
                                }
                                state.last_update_check = Some(result);
                            }
                            Err(_) => {
                                if matches!(state.update_state, UpdateState::Checking) {
                                    state.update_state =
                                        UpdateState::UpToDate(state.current_version.clone());
                                }
                            }
                        }
                        cx.notify();
                    })
                    .is_ok();

                if !should_continue {
                    break;
                }

                background
                    .timer(Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS))
                    .await;
            }
        })
        .detach();
    }

    /// Trigger an immediate update check against the Forgejo releases endpoint.
    pub fn check_updates_now(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        self.update_state = UpdateState::Checking;
        cx.notify();

        let current_version = self.current_version.clone();
        let background = cx.background_executor().clone();

        cx.spawn(async move |this, cx| {
            let check_res = background
                .spawn(async move { check_for_updates(&current_version) })
                .await;

            let _ = this.update(cx, |state, cx| {
                match check_res {
                    Ok(result) => {
                        if result.update_available {
                            state.update_state = UpdateState::Available {
                                version: result.latest_version.clone(),
                                install_kind: result.install_kind,
                            };
                        } else {
                            state.update_state =
                                UpdateState::UpToDate(result.latest_version.clone());
                        }
                        state.last_update_check = Some(result);
                    }
                    Err(err) => {
                        state.update_state = UpdateState::Error("Update check failed".to_string());
                        state.status_message = Some(format!("Update check failed: {}", err));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Download and install the latest available update for Windows MSI/EXE, Linux Native, or Flatpak.
    pub fn apply_update_now(&mut self, cx: &mut Context<Self>) {
        let Some(check) = self.last_update_check.clone() else {
            self.check_updates_now(cx);
            return;
        };

        let ver = check.latest_version.clone();
        self.update_state = UpdateState::Downloading(ver.clone());
        cx.notify();

        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let apply_res = background
                .spawn(async move { download_and_apply_update(&check) })
                .await;

            let _ = this.update(cx, |state, cx| {
                match apply_res {
                    Ok(msg) => {
                        state.update_state = UpdateState::ReadyToRestart(ver);
                        state.status_message = Some(msg);
                    }
                    Err(err) => {
                        state.update_state = UpdateState::Error("Update failed".to_string());
                        state.error_message = Some(format!("Update error: {}", err));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Open native OS file picker dialog to load a database, Parquet, or CSV file.
    pub fn prompt_open_file(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open SQLite, DuckDB, Parquet, or CSV File".into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(first) = paths.into_iter().next() {
                    let path_str = first.to_string_lossy().to_string();
                    let _ = this.update(cx, |state, cx| {
                        state.load_file(path_str, cx);
                    });
                }
            }
        })
        .detach();
    }

    /// Open native OS file picker dialog to select a baseline database file for schema comparison.
    pub fn prompt_compare_file(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Baseline Database".into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(first) = paths.into_iter().next() {
                    let path_str = first.to_string_lossy().to_string();
                    let lower = path_str.to_ascii_lowercase();
                    let is_duck = lower.ends_with(".duckdb")
                        || lower.ends_with(".ddb")
                        || lower.ends_with(".parquet")
                        || lower.ends_with(".pq")
                        || lower.ends_with(".csv")
                        || lower.ends_with(".tsv");

                    let compare_res: Result<DatabaseSchema, String> = async {
                        let mut driver: Box<dyn DatabaseDriver> = if is_duck {
                            Box::new(DuckDbDriver::new())
                        } else {
                            Box::new(SqliteDriver::new())
                        };
                        driver
                            .connect(&path_str)
                            .await
                            .map_err(|e| format!("Connection error: {}", e))?;
                        driver
                            .introspect_schema()
                            .await
                            .map_err(|e| format!("Introspection error: {}", e))
                    }
                    .await;

                    let _ = this.update(cx, |state, cx| {
                        match compare_res {
                            Ok(baseline_schema) => {
                                let short_name = std::path::Path::new(&path_str)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&path_str)
                                    .to_string();
                                state.snapshot_schema = Some(baseline_schema);
                                state.snapshot_label = Some(short_name.clone());
                                state.recompute_diff();
                                state.status_message = Some(format!("Baseline: {}", short_name));
                            }
                            Err(err) => {
                                state.error_message = Some(err);
                            }
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Load a database, Parquet, or CSV/TSV file by path.
    pub fn load_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        if path.to_ascii_lowercase().ends_with(".json") {
            self.load_theme_from_file(&path, cx);
            return;
        }

        self.record_recent_file(&path);
        self.is_loading_schema = true;
        self.error_message = None;
        self.selected_row = None;
        self.sort_column = None;
        self.file_path = Some(path.clone());

        let lower = path.to_ascii_lowercase();
        let is_parquet = lower.ends_with(".parquet") || lower.ends_with(".pq");
        let is_csv = lower.ends_with(".csv") || lower.ends_with(".tsv");
        let is_duckdb_engine =
            lower.ends_with(".duckdb") || lower.ends_with(".ddb") || is_parquet || is_csv;

        let engine = if is_parquet {
            "Parquet"
        } else if is_csv {
            "CSV"
        } else if is_duckdb_engine {
            "DuckDB"
        } else {
            "SQLite"
        };
        self.engine_type = engine.to_string();
        self.diff_dialect = if is_duckdb_engine {
            SqlDialect::DuckDb
        } else {
            SqlDialect::Sqlite
        };

        cx.spawn(async move |this, cx| {
            let connect_result: Result<(Box<dyn DatabaseDriver>, DatabaseSchema), String> = async {
                let mut driver: Box<dyn DatabaseDriver> = if is_duckdb_engine {
                    Box::new(DuckDbDriver::new())
                } else {
                    Box::new(SqliteDriver::new())
                };

                driver
                    .connect(&path)
                    .await
                    .map_err(|e| format!("Connection error: {}", e))?;

                let schema = driver
                    .introspect_schema()
                    .await
                    .map_err(|e| format!("Introspection error: {}", e))?;

                Ok((driver, schema))
            }
            .await;

            let _ = this.update(cx, |state, cx| {
                state.is_loading_schema = false;
                match connect_result {
                    Ok((driver, schema)) => {
                        let tbl_count = schema.tables.len();
                        let first_table_name = schema.tables.first().map(|t| t.name.clone());
                        state.schema = Some(schema);
                        state.active_driver = Some(Arc::new(futures::lock::Mutex::new(driver)));
                        state.recompute_diff();
                        state.status_message = Some(format!(
                            "{} • {} tables",
                            state.short_file_name(),
                            tbl_count
                        ));
                        if let Some(first_tbl) = first_table_name {
                            state.selected_table = Some(first_tbl.clone());
                            let sql = format!(
                                "SELECT * FROM \"{}\" LIMIT {};",
                                first_tbl, state.default_row_limit
                            );
                            state.set_query_buffer_with_cursor(sql);
                            if state.auto_query_on_select {
                                state.execute_query(cx);
                            }
                        }
                    }
                    Err(err) => {
                        state.error_message = Some(err);
                        state.status_message = Some("Connection error".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Re-introspect the currently connected database schema.
    pub fn refresh_schema(&mut self, cx: &mut Context<Self>) {
        let Some(driver_arc) = self.active_driver.clone() else {
            return;
        };
        self.is_loading_schema = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let schema_res = {
                let driver = driver_arc.lock().await;
                driver.introspect_schema().await
            };
            let _ = this.update(cx, |state, cx| {
                state.is_loading_schema = false;
                match schema_res {
                    Ok(new_schema) => {
                        let count = new_schema.tables.len();
                        state.schema = Some(new_schema);
                        state.recompute_diff();
                        state.status_message =
                            Some(format!("Refreshed • {} tables", count));
                    }
                    Err(e) => {
                        state.error_message = Some(format!("Schema refresh error: {}", e));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Execute the SQL query currently present in the buffer, and refresh schema if DDL was executed.
    pub fn execute_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.query_buffer.trim().to_string();
        if sql.is_empty() {
            return;
        }

        let driver_arc = match &self.active_driver {
            Some(d) => d.clone(),
            None => {
                self.error_message = Some("No active connection".to_string());
                cx.notify();
                return;
            }
        };

        // Record in query history (deduplicating most recent)
        if self.query_history.first().map(|s| s.as_str()) != Some(sql.as_str()) {
            self.query_history.insert(0, sql.clone());
            if self.query_history.len() > 20 {
                self.query_history.truncate(20);
            }
        }
        self.history_index = None;

        self.is_executing = true;
        self.error_message = None;
        self.selected_row = None;
        cx.notify();

        let upper_sql = sql.to_uppercase();
        let is_ddl = upper_sql.contains("CREATE ")
            || upper_sql.contains("ALTER ")
            || upper_sql.contains("DROP ");

        cx.spawn(async move |this, cx| {
            let start = Instant::now();
            let (query_result, updated_schema) = {
                let driver = driver_arc.lock().await;
                let res = driver.execute_query(&sql).await;
                let schema = if is_ddl && res.is_ok() {
                    driver.introspect_schema().await.ok()
                } else {
                    None
                };
                (res, schema)
            };
            let elapsed_ms = start.elapsed().as_millis();

            let _ = this.update(cx, |state, cx| {
                state.is_executing = false;
                state.execution_time_ms = Some(elapsed_ms);

                if let Some(new_schema) = updated_schema {
                    state.schema = Some(new_schema);
                    state.recompute_diff();
                }

                match query_result {
                    Ok(batches) => {
                        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                        state.query_results = Some(batches);
                        state.result_seq = state.result_seq.wrapping_add(1);
                        state.status_message =
                            Some(format!("{} rows • {} ms", total_rows, elapsed_ms));
                    }
                    Err(err) => {
                        state.error_message = Some(format!("{}", err));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn set_query_buffer_with_cursor(&mut self, sql: String) {
        self.query_cursor = sql.chars().count();
        self.query_buffer = sql;
    }

    /// Set SQL query text and execute immediately.
    pub fn set_query_and_run(&mut self, sql: String, cx: &mut Context<Self>) {
        self.set_query_buffer_with_cursor(sql);
        self.active_tab = ActiveTab::QueryAndData;
        self.execute_query(cx);
    }

    /// Toggle sorting by a column in the data grid.
    pub fn toggle_sort_column(&mut self, col_name: String, cx: &mut Context<Self>) {
        if self.sort_column.as_deref() == Some(col_name.as_str()) {
            if self.sort_ascending {
                self.sort_ascending = false;
            } else {
                self.sort_column = None;
                self.sort_ascending = true;
            }
        } else {
            self.sort_column = Some(col_name);
            self.sort_ascending = true;
        }

        if let Some(tbl) = self.selected_table.clone() {
            let escaped_tbl = tbl.replace('"', "\"\"");
            let sql = if let Some(ref col) = self.sort_column {
                let escaped_col = col.replace('"', "\"\"");
                let dir = if self.sort_ascending { "ASC" } else { "DESC" };
                format!(
                    "SELECT * FROM \"{}\" ORDER BY \"{}\" {} LIMIT {};",
                    escaped_tbl, escaped_col, dir, self.default_row_limit
                )
            } else {
                format!(
                    "SELECT * FROM \"{}\" LIMIT {};",
                    escaped_tbl, self.default_row_limit
                )
            };
            self.set_query_and_run(sql, cx);
        } else {
            cx.notify();
        }
    }

    /// Insert text at the current cursor position in the SQL query buffer.
    pub fn append_to_query(&mut self, text: &str, cx: &mut Context<Self>) {
        let char_count = self.query_buffer.chars().count();
        let cursor = self.query_cursor.min(char_count);
        let byte_idx = self
            .query_buffer
            .char_indices()
            .nth(cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.query_buffer.len());
        self.query_buffer.insert_str(byte_idx, text);
        self.query_cursor = cursor + text.chars().count();
        cx.notify();
    }

    /// Handle backspace at the current cursor position in the SQL query buffer.
    pub fn backspace_query(&mut self, cx: &mut Context<Self>) {
        let char_count = self.query_buffer.chars().count();
        let cursor = self.query_cursor.min(char_count);
        if cursor > 0 {
            if let Some((byte_idx, _)) = self.query_buffer.char_indices().nth(cursor - 1) {
                self.query_buffer.remove(byte_idx);
                self.query_cursor = cursor - 1;
                cx.notify();
            }
        }
    }

    /// Handle delete (forward delete) at the current cursor position in the SQL query buffer.
    pub fn delete_at_cursor(&mut self, cx: &mut Context<Self>) {
        let char_count = self.query_buffer.chars().count();
        let cursor = self.query_cursor.min(char_count);
        if cursor < char_count {
            if let Some((byte_idx, _)) = self.query_buffer.char_indices().nth(cursor) {
                self.query_buffer.remove(byte_idx);
                cx.notify();
            }
        }
    }

    pub fn move_cursor_left(&mut self, cx: &mut Context<Self>) {
        if self.query_cursor > 0 {
            self.query_cursor -= 1;
            cx.notify();
        }
    }

    pub fn move_cursor_right(&mut self, cx: &mut Context<Self>) {
        let max_len = self.query_buffer.chars().count();
        if self.query_cursor < max_len {
            self.query_cursor += 1;
            cx.notify();
        }
    }

    pub fn move_cursor_home(&mut self, cx: &mut Context<Self>) {
        self.query_cursor = 0;
        cx.notify();
    }

    pub fn move_cursor_end(&mut self, cx: &mut Context<Self>) {
        self.query_cursor = self.query_buffer.chars().count();
        cx.notify();
    }

    /// Cycle through executed query history.
    pub fn cycle_query_history(&mut self, cx: &mut Context<Self>) {
        if self.query_history.is_empty() {
            return;
        }
        let next_idx = match self.history_index {
            None => 0,
            Some(i) => (i + 1) % self.query_history.len(),
        };
        self.history_index = Some(next_idx);
        let sql = self.query_history[next_idx].clone();
        self.set_query_buffer_with_cursor(sql);
        cx.notify();
    }

    /// Format the current SQL query buffer cleanly with uppercase keywords and clause line breaks.
    pub fn format_query_buffer(&mut self, cx: &mut Context<Self>) {
        let raw = self.query_buffer.trim();
        if raw.is_empty() {
            return;
        }
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        let major_clauses = [
            "SELECT", "FROM", "WHERE", "GROUP", "ORDER", "LIMIT", "HAVING", "LEFT", "INNER",
            "RIGHT", "JOIN", "VALUES", "SET", "UNION",
        ];
        let uppercase_kw = [
            "SELECT", "FROM", "WHERE", "GROUP", "BY", "ORDER", "ASC", "DESC", "LIMIT", "OFFSET",
            "HAVING", "JOIN", "LEFT", "INNER", "RIGHT", "ON", "AS", "AND", "OR", "NOT", "IN",
            "IS", "NULL", "COUNT", "SUM", "AVG", "MIN", "MAX", "DISTINCT", "INSERT", "INTO",
            "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "ALTER", "DROP", "VIEW",
            "INDEX", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "PRAGMA", "DESCRIBE", "SHOW",
            "WITH", "CASE", "WHEN", "THEN", "ELSE", "END",
        ];

        let mut out = String::new();
        for (i, tok) in tokens.iter().enumerate() {
            let clean = tok.trim_end_matches(';').trim_end_matches(',');
            let has_semi = tok.ends_with(';');
            let has_comma = tok.ends_with(',');
            let upper = clean.to_ascii_uppercase();

            if i > 0 && major_clauses.contains(&upper.as_str()) {
                out.push('\n');
            } else if i > 0 {
                out.push(' ');
            }

            if uppercase_kw.contains(&upper.as_str()) {
                out.push_str(&upper);
            } else {
                out.push_str(clean);
            }

            if has_comma {
                out.push(',');
            }
            if has_semi {
                out.push(';');
            }
        }
        self.set_query_buffer_with_cursor(out);
        cx.notify();
    }

    /// Clear the SQL query buffer.
    pub fn clear_query(&mut self, cx: &mut Context<Self>) {
        self.query_buffer.clear();
        self.query_cursor = 0;
        self.error_message = None;
        cx.notify();
    }

    /// Select a table from the sidebar and query its rows.
    pub fn select_table(&mut self, table_name: String, cx: &mut Context<Self>) {
        self.selected_table = Some(table_name.clone());
        self.sort_column = None;
        self.sort_ascending = true;
        let sql = format!(
            "SELECT * FROM \"{}\" LIMIT {};",
            table_name.replace('"', "\"\""),
            self.default_row_limit
        );
        self.set_query_buffer_with_cursor(sql);
        self.active_tab = ActiveTab::QueryAndData;
        if self.auto_query_on_select {
            self.execute_query(cx);
        } else {
            cx.notify();
        }
    }

    /// Serialize the current query results into RFC-4180 CSV format.
    pub fn results_to_csv(&self) -> Option<String> {
        let batches = self.query_results.as_ref()?;
        let first = batches.first()?;
        let schema = first.schema();
        let fields = schema.fields();

        let mut csv = String::new();
        // Header row
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                csv.push(',');
            }
            let name = f.name();
            if name.contains(',') || name.contains('"') || name.contains('\n') {
                csv.push('"');
                csv.push_str(&name.replace('"', "\"\""));
                csv.push('"');
            } else {
                csv.push_str(name);
            }
        }
        csv.push('\n');

        // Data rows
        for batch in batches {
            for r in 0..batch.num_rows() {
                for c in 0..batch.num_columns() {
                    if c > 0 {
                        csv.push(',');
                    }
                    let col = batch.column(c);
                    let (val, is_null, _) = format_arrow_cell(col, r);
                    if is_null {
                        continue;
                    }
                    if val.contains(',') || val.contains('"') || val.contains('\n') {
                        csv.push('"');
                        csv.push_str(&val.replace('"', "\"\""));
                        csv.push('"');
                    } else {
                        csv.push_str(&val);
                    }
                }
                csv.push('\n');
            }
        }
        Some(csv)
    }

    /// Serialize the current query results into a pretty JSON array of objects.
    pub fn results_to_json(&self) -> Option<String> {
        let batches = self.query_results.as_ref()?;
        let first = batches.first()?;
        let schema = first.schema();
        let fields = schema.fields();

        let mut rows_json = Vec::new();
        for batch in batches {
            for r in 0..batch.num_rows() {
                let mut map = serde_json::Map::new();
                for c in 0..batch.num_columns() {
                    let col_name = fields[c].name().clone();
                    let col = batch.column(c);
                    let (val, is_null, is_numeric) = format_arrow_cell(col, r);
                    let json_val = if is_null {
                        serde_json::Value::Null
                    } else if is_numeric {
                        if let Ok(i) = val.parse::<i64>() {
                            serde_json::Value::Number(i.into())
                        } else if let Ok(f) = val.parse::<f64>() {
                            serde_json::Number::from_f64(f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::String(val))
                        } else {
                            serde_json::Value::String(val)
                        }
                    } else if val == "true" || val == "false" {
                        serde_json::Value::Bool(val == "true")
                    } else {
                        serde_json::Value::String(val)
                    };
                    map.insert(col_name, json_val);
                }
                rows_json.push(serde_json::Value::Object(map));
            }
        }
        serde_json::to_string_pretty(&rows_json).ok()
    }

    /// Copy the current query results as CSV to the system clipboard.
    pub fn copy_results_csv(&mut self, cx: &mut Context<Self>) {
        if let Some(csv) = self.results_to_csv() {
            cx.write_to_clipboard(ClipboardItem::new_string(csv));
            self.status_message = Some("Copied results as CSV".to_string());
            cx.notify();
        }
    }

    /// Copy the current query results as JSON to the system clipboard.
    pub fn copy_results_json(&mut self, cx: &mut Context<Self>) {
        if let Some(json) = self.results_to_json() {
            cx.write_to_clipboard(ClipboardItem::new_string(json));
            self.status_message = Some("Copied results as JSON".to_string());
            cx.notify();
        }
    }

    /// Prompt user to save current query results to a `.csv` file.
    pub fn prompt_export_csv(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let Some(csv) = self.results_to_csv() else {
            self.error_message = Some("No query results to export".to_string());
            cx.notify();
            return;
        };
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_name = self
            .selected_table
            .as_ref()
            .map(|t| format!("{}.csv", t))
            .unwrap_or_else(|| "query_results.csv".to_string());
        let receiver = cx.prompt_for_new_path(&dir, Some(&default_name));

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let _ = this.update(cx, |state, cx| {
                    match std::fs::write(&path, &csv) {
                        Ok(()) => {
                            state.status_message =
                                Some(format!("Exported CSV to {}", path.display()));
                        }
                        Err(e) => {
                            state.error_message = Some(format!("CSV export failed: {}", e));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Prompt user to save current query results to a `.json` file.
    pub fn prompt_export_json(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        let Some(json) = self.results_to_json() else {
            self.error_message = Some("No query results to export".to_string());
            cx.notify();
            return;
        };
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_name = self
            .selected_table
            .as_ref()
            .map(|t| format!("{}.json", t))
            .unwrap_or_else(|| "query_results.json".to_string());
        let receiver = cx.prompt_for_new_path(&dir, Some(&default_name));

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let _ = this.update(cx, |state, cx| {
                    match std::fs::write(&path, &json) {
                        Ok(()) => {
                            state.status_message =
                                Some(format!("Exported JSON to {}", path.display()));
                        }
                        Err(e) => {
                            state.error_message = Some(format!("JSON export failed: {}", e));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Take a snapshot of the current schema for diffing.
    pub fn take_schema_snapshot(&mut self, cx: &mut Context<Self>) {
        self.active_top_menu = None;
        if let Some(ref current) = self.schema {
            self.snapshot_schema = Some(current.clone());
            self.snapshot_label = Some(self.short_file_name());
            self.recompute_diff();
            self.status_message = Some(format!("Snapshot captured: {}", self.short_file_name()));
            cx.notify();
        }
    }

    /// Recompute the schema diff between snapshot and current schema.
    pub fn recompute_diff(&mut self) {
        if let (Some(base), Some(current)) = (&self.snapshot_schema, &self.schema) {
            let diff = diff_schemas(base, current);
            self.diff_migration_sql = diff.generate_migration_sql(self.diff_dialect);
            self.schema_diff = Some(diff);
        }
    }

    /// Load the generated migration SQL into the query editor so it can be inspected or run.
    pub fn load_migration_into_editor(&mut self, cx: &mut Context<Self>) {
        if !self.diff_migration_sql.is_empty() {
            let sql = self.diff_migration_sql.join("\n\n");
            self.set_query_buffer_with_cursor(sql);
            self.active_tab = ActiveTab::QueryAndData;
            self.view_transition_seq = self.view_transition_seq.wrapping_add(1);
            cx.notify();
        }
    }

    /// Switch active tab (Data Grid vs Schema Diff).
    pub fn set_active_tab(&mut self, tab: ActiveTab, cx: &mut Context<Self>) {
        if self.active_tab != tab {
            self.active_tab = tab;
            self.view_transition_seq = self.view_transition_seq.wrapping_add(1);
        }
        self.active_top_menu = None;
        cx.notify();
    }

    /// Switch SQL migration dialect.
    pub fn set_diff_dialect(&mut self, dialect: SqlDialect, cx: &mut Context<Self>) {
        self.diff_dialect = dialect;
        if let Some(ref diff) = self.schema_diff {
            self.diff_migration_sql = diff.generate_migration_sql(dialect);
        }
        cx.notify();
    }

    /// Return a compact display name for the currently connected file.
    pub fn short_file_name(&self) -> String {
        match &self.file_path {
            Some(p) if p.starts_with(":memory:") => ":memory:".to_string(),
            Some(p) => std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
                .to_string(),
            None => String::new(),
        }
    }
}
