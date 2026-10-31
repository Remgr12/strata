use arrow::record_batch::RecordBatch;
use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;
use std::time::Instant;
use strata_core::{
    diff_schemas, DatabaseDriver, DatabaseSchema, DuckDbDriver, SchemaDiff, SqlDialect,
    SqliteDriver,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    QueryAndData,
    SchemaDiff,
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

    // Diff view state
    pub snapshot_schema: Option<DatabaseSchema>,
    pub schema_diff: Option<SchemaDiff>,
    pub diff_migration_sql: Vec<String>,
    pub diff_dialect: SqlDialect,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_driver: None,
            file_path: None,
            engine_type: "None".to_string(),
            schema: None,
            query_buffer: "SELECT 1 AS ready, 'Welcome to Strata' AS message;".to_string(),
            query_results: None,
            execution_time_ms: None,
            is_executing: false,
            is_loading_schema: false,
            status_message: Some("Ready - Drag and drop a database file to open".to_string()),
            error_message: None,
            active_tab: ActiveTab::QueryAndData,
            selected_table: None,
            snapshot_schema: None,
            schema_diff: None,
            diff_migration_sql: Vec::new(),
            diff_dialect: SqlDialect::Sqlite,
        }
    }

    /// Load a database or parquet file by path.
    pub fn load_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.is_loading_schema = true;
        self.error_message = None;
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
                        state.schema = Some(schema);
                        state.active_driver = Some(Arc::new(futures::lock::Mutex::new(driver)));
                        state.status_message = Some(format!(
                            "Loaded {} successfully ({} tables found)",
                            path, tbl_count
                        ));
                        if tbl_count > 0 {
                            if let Some(first_tbl) = state.schema.as_ref().unwrap().tables.first() {
                                state.selected_table = Some(first_tbl.name.clone());
                                state.query_buffer = format!(
                                    "SELECT * FROM \"{}\" LIMIT 100;",
                                    first_tbl.name
                                );
                            }
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

    /// Execute the SQL query currently present in the buffer.
    pub fn execute_query(&mut self, cx: &mut Context<Self>) {
        let sql = self.query_buffer.trim().to_string();
        if sql.is_empty() {
            self.error_message = Some("Cannot execute an empty query".to_string());
            cx.notify();
            return;
        }

        let driver_arc = match &self.active_driver {
            Some(d) => d.clone(),
            None => {
                self.error_message = Some("No database connected. Please drag and drop a file or create in-memory database.".to_string());
                cx.notify();
                return;
            }
        };

        self.is_executing = true;
        self.error_message = None;
        self.status_message = Some("Executing query...".to_string());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let start = Instant::now();
            let query_result = {
                let driver = driver_arc.lock().await;
                driver.execute_query(&sql).await
            };
            let elapsed_ms = start.elapsed().as_millis();

            let _ = this.update(cx, |state, cx| {
                state.is_executing = false;
                state.execution_time_ms = Some(elapsed_ms);

                match query_result {
                    Ok(batches) => {
                        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                        state.query_results = Some(batches);
                        state.status_message = Some(format!(
                            "Query executed in {}ms ({} rows returned)",
                            elapsed_ms, total_rows
                        ));
                    }
                    Err(err) => {
                        state.error_message = Some(format!("Execution Error: {}", err));
                        state.status_message = Some("Query failed".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Set SQL query text and optionally execute immediately.
    #[allow(dead_code)]
    pub fn set_query_and_run(&mut self, sql: String, cx: &mut Context<Self>) {
        self.query_buffer = sql;
        self.execute_query(cx);
    }

    /// Select a table from the sidebar.
    pub fn select_table(&mut self, table_name: String, cx: &mut Context<Self>) {
        self.selected_table = Some(table_name.clone());
        self.query_buffer = format!("SELECT * FROM \"{}\" LIMIT 100;", table_name);
        self.execute_query(cx);
    }

    /// Take a snapshot of the current schema for diffing.
    pub fn take_schema_snapshot(&mut self, cx: &mut Context<Self>) {
        if let Some(ref current) = self.schema {
            self.snapshot_schema = Some(current.clone());
            self.status_message = Some("Schema snapshot captured for diff comparison".to_string());
            self.recompute_diff();
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
}
