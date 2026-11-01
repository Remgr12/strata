use arrow::record_batch::RecordBatch;
use gpui::prelude::*;
use gpui::*;
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

pub struct AppState {
    pub active_driver: Option<Arc<futures::lock::Mutex<Box<dyn DatabaseDriver>>>>,
    pub file_path: Option<String>,
    pub engine_type: String,
    pub schema: Option<DatabaseSchema>,
    pub query_buffer: String,
    pub query_results: Option<Vec<RecordBatch>>,
    pub execution_time_ms: Option<u128>,
    pub is_executing: bool,
    pub is_loading_schema: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub active_tab: ActiveTab,
    pub selected_table: Option<String>,
    pub selected_row: Option<usize>,

    // Diff view state
    pub snapshot_schema: Option<DatabaseSchema>,
    pub snapshot_label: Option<String>,
    pub schema_diff: Option<SchemaDiff>,
    pub diff_migration_sql: Vec<String>,
    pub diff_dialect: SqlDialect,

    // Auto-updater state
    pub current_version: String,
    pub install_kind: InstallKind,
    pub update_state: UpdateState,
    pub last_update_check: Option<UpdateCheckResult>,
}

impl AppState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let install_kind = InstallKind::detect();

        let mut state = Self {
            active_driver: None,
            file_path: None,
            engine_type: "None".to_string(),
            schema: None,
            query_buffer: "SELECT 1 AS id, 'Strata Workspace' AS workspace, 'Ready' AS status;"
                .to_string(),
            query_results: None,
            execution_time_ms: None,
            is_executing: false,
            is_loading_schema: false,
            status_message: Some("Open or drop a SQLite, DuckDB, or Parquet file to begin".to_string()),
            error_message: None,
            active_tab: ActiveTab::QueryAndData,
            selected_table: None,
            selected_row: None,
            snapshot_schema: None,
            snapshot_label: None,
            schema_diff: None,
            diff_migration_sql: Vec::new(),
            diff_dialect: SqlDialect::Sqlite,
            current_version,
            install_kind,
            update_state: UpdateState::Idle,
            last_update_check: None,
        };

        state.start_periodic_update_checker(cx);
        state
    }

    /// Periodically checks the Forgejo Releases API for newer versions in the background.
    pub fn start_periodic_update_checker(&mut self, cx: &mut Context<Self>) {
        let current_version = self.current_version.clone();
        let background = cx.background_executor().clone();

        cx.spawn(async move |this, cx| {
            // Brief initial delay after startup before first check
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
        self.update_state = UpdateState::Checking;
        self.status_message = Some(format!(
            "Checking releases for {} updates...",
            self.install_kind.label()
        ));
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
                            state.status_message = Some(format!(
                                "Update v{} is available for {}",
                                result.latest_version,
                                result.install_kind.label()
                            ));
                        } else {
                            state.update_state =
                                UpdateState::UpToDate(result.latest_version.clone());
                            state.status_message = Some(format!(
                                "Up to date (v{}, {})",
                                state.current_version,
                                state.install_kind.label()
                            ));
                        }
                        state.last_update_check = Some(result);
                    }
                    Err(err) => {
                        state.update_state = UpdateState::Error("Update check failed".to_string());
                        state.status_message =
                            Some(format!("Could not reach releases page: {}", err));
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
        self.status_message = Some(format!(
            "Downloading Strata v{} ({})...",
            ver,
            check.install_kind.label()
        ));
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

    /// Open native OS file picker dialog to load a database file.
    pub fn prompt_open_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Database or Parquet File".into()),
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
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Baseline Database to Compare".into()),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(first) = paths.into_iter().next() {
                    let path_str = first.to_string_lossy().to_string();
                    let is_duck = path_str.ends_with(".duckdb")
                        || path_str.ends_with(".ddb")
                        || path_str.ends_with(".parquet")
                        || path_str.ends_with(".pq");

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
                                state.status_message = Some(format!(
                                    "Comparing active schema against baseline '{}'",
                                    short_name
                                ));
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

    /// Load a database or parquet file by path.
    pub fn load_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.is_loading_schema = true;
        self.error_message = None;
        self.selected_row = None;
        self.status_message = Some(format!("Connecting to {}...", path));
        self.file_path = Some(path.clone());

        let is_duckdb_or_parquet = path.ends_with(".duckdb")
            || path.ends_with(".ddb")
            || path.ends_with(".parquet")
            || path.ends_with(".pq");

        let engine = if path.ends_with(".parquet") || path.ends_with(".pq") {
            "Parquet"
        } else if is_duckdb_or_parquet {
            "DuckDB"
        } else {
            "SQLite"
        };
        self.engine_type = engine.to_string();
        self.diff_dialect = if is_duckdb_or_parquet {
            SqlDialect::DuckDb
        } else {
            SqlDialect::Sqlite
        };

        cx.spawn(async move |this, cx| {
            let connect_result: Result<(Box<dyn DatabaseDriver>, DatabaseSchema), String> = async {
                let mut driver: Box<dyn DatabaseDriver> = if is_duckdb_or_parquet {
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
                            "Connected to {} ({} tables)",
                            state.short_file_name(),
                            tbl_count
                        ));
                        if let Some(first_tbl) = first_table_name {
                            state.selected_table = Some(first_tbl.clone());
                            state.query_buffer =
                                format!("SELECT * FROM \"{}\" LIMIT 100;", first_tbl);
                            state.execute_query(cx);
                        }
                    }
                    Err(err) => {
                        state.error_message = Some(err);
                        state.status_message = Some("Connection failed".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Initialize an interactive in-memory demo database with sample tables and data.
    pub fn load_demo_workspace(&mut self, cx: &mut Context<Self>) {
        self.is_loading_schema = true;
        self.error_message = None;
        self.selected_row = None;
        self.file_path = Some(":memory: (sample)".to_string());
        self.engine_type = "SQLite".to_string();
        self.diff_dialect = SqlDialect::Sqlite;
        self.status_message = Some("Initializing sample database...".to_string());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let init_res: Result<
                (Box<dyn DatabaseDriver>, DatabaseSchema, DatabaseSchema),
                String,
            > = async {
                let mut driver: Box<dyn DatabaseDriver> = Box::new(SqliteDriver::new());
                driver
                    .connect(":memory:")
                    .await
                    .map_err(|e| format!("Connection error: {}", e))?;

                let seed_statements = [
                    "CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, plan TEXT NOT NULL, created_at TEXT);",
                    "CREATE TABLE products (id INTEGER PRIMARY KEY, sku TEXT NOT NULL, title TEXT NOT NULL, price_usd REAL NOT NULL, in_stock INTEGER NOT NULL);",
                    "INSERT INTO customers VALUES (1, 'Elena Rostova', 'elena@soventa.tech', 'Enterprise', '2026-09-14'), (2, 'Marcus Vance', 'mvance@acme.io', 'Pro', '2026-10-02'), (3, 'Sora Takahashi', 'sora@kumo.jp', 'Enterprise', '2026-10-19'), (4, 'Liam O''Connor', 'liam@dublin.dev', 'Starter', '2026-10-28');",
                    "INSERT INTO products VALUES (101, 'STR-ENT-01', 'Strata Enterprise Node', 240.00, 1), (102, 'STR-PRO-01', 'Strata Pro Seat', 49.00, 1), (103, 'STR-ARC-02', 'Arrow Columnar Connector', 99.00, 1);",
                ];

                for stmt in seed_statements {
                    driver
                        .execute_query(stmt)
                        .await
                        .map_err(|e| format!("Seed error: {}", e))?;
                }

                // Capture baseline snapshot before adding orders table so Schema Diff is immediately demonstrable
                let baseline = driver
                    .introspect_schema()
                    .await
                    .map_err(|e| format!("Introspect error: {}", e))?;

                let post_snapshot_statements = [
                    "CREATE TABLE orders (id INTEGER PRIMARY KEY, customer_id INTEGER NOT NULL, product_id INTEGER NOT NULL, quantity INTEGER NOT NULL, total_usd REAL NOT NULL, status TEXT NOT NULL);",
                    "INSERT INTO orders VALUES (1001, 1, 101, 3, 720.00, 'completed'), (1002, 2, 102, 5, 245.00, 'completed'), (1003, 3, 103, 2, 198.00, 'processing'), (1004, 1, 102, 10, 490.00, 'completed');",
                ];

                for stmt in post_snapshot_statements {
                    driver
                        .execute_query(stmt)
                        .await
                        .map_err(|e| format!("Seed error: {}", e))?;
                }

                let schema = driver
                    .introspect_schema()
                    .await
                    .map_err(|e| format!("Introspection error: {}", e))?;

                Ok((driver, baseline, schema))
            }
            .await;

            let _ = this.update(cx, |state, cx| {
                state.is_loading_schema = false;
                match init_res {
                    Ok((driver, baseline, schema)) => {
                        state.snapshot_schema = Some(baseline);
                        state.snapshot_label = Some("v1 baseline snapshot".to_string());
                        state.schema = Some(schema);
                        state.active_driver = Some(Arc::new(futures::lock::Mutex::new(driver)));
                        state.recompute_diff();
                        state.selected_table = Some("customers".to_string());
                        state.query_buffer =
                            "SELECT id, name, email, plan, created_at FROM customers ORDER BY id;"
                                .to_string();
                        state.status_message =
                            Some("Sample workspace loaded (3 tables)".to_string());
                        state.execute_query(cx);
                    }
                    Err(err) => {
                        state.error_message = Some(err);
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
            self.error_message = Some("Enter a SQL statement to execute".to_string());
            cx.notify();
            return;
        }

        let driver_arc = match &self.active_driver {
            Some(d) => d.clone(),
            None => {
                self.error_message = Some(
                    "No database connected. Open a file or click 'Sample DB' to start.".to_string(),
                );
                cx.notify();
                return;
            }
        };

        self.is_executing = true;
        self.error_message = None;
        self.selected_row = None;
        self.status_message = Some("Running query...".to_string());
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
                        state.status_message =
                            Some(format!("{} rows returned in {} ms", total_rows, elapsed_ms));
                    }
                    Err(err) => {
                        state.error_message = Some(format!("{}", err));
                        state.status_message = Some("Query failed".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Set SQL query text and execute immediately.
    pub fn set_query_and_run(&mut self, sql: String, cx: &mut Context<Self>) {
        self.query_buffer = sql;
        self.active_tab = ActiveTab::QueryAndData;
        self.execute_query(cx);
    }

    /// Append or insert text into the SQL query buffer.
    pub fn append_to_query(&mut self, text: &str, cx: &mut Context<Self>) {
        self.query_buffer.push_str(text);
        cx.notify();
    }

    /// Handle backspace in the SQL query buffer.
    pub fn backspace_query(&mut self, cx: &mut Context<Self>) {
        self.query_buffer.pop();
        cx.notify();
    }

    /// Clear the SQL query buffer.
    pub fn clear_query(&mut self, cx: &mut Context<Self>) {
        self.query_buffer.clear();
        self.error_message = None;
        cx.notify();
    }

    /// Select a table from the sidebar and query its rows.
    pub fn select_table(&mut self, table_name: String, cx: &mut Context<Self>) {
        self.selected_table = Some(table_name.clone());
        self.query_buffer = format!("SELECT * FROM \"{}\" LIMIT 100;", table_name);
        self.active_tab = ActiveTab::QueryAndData;
        self.execute_query(cx);
    }

    /// Take a snapshot of the current schema for diffing.
    pub fn take_schema_snapshot(&mut self, cx: &mut Context<Self>) {
        if let Some(ref current) = self.schema {
            self.snapshot_schema = Some(current.clone());
            self.snapshot_label = Some(format!("Snapshot ({})", self.short_file_name()));
            self.status_message =
                Some("Baseline schema snapshot captured for comparison".to_string());
            self.recompute_diff();
            cx.notify();
        } else {
            self.error_message =
                Some("Connect a database before capturing a schema snapshot".to_string());
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
            self.query_buffer = self.diff_migration_sql.join("\n\n");
            self.active_tab = ActiveTab::QueryAndData;
            self.status_message = Some("Loaded migration script into SQL editor".to_string());
            cx.notify();
        }
    }

    /// Switch active tab (Data Grid vs Schema Diff).
    pub fn set_active_tab(&mut self, tab: ActiveTab, cx: &mut Context<Self>) {
        self.active_tab = tab;
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
            Some(p) if p.starts_with(":memory:") => p.clone(),
            Some(p) => std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
                .to_string(),
            None => "No database".to_string(),
        }
    }
}
