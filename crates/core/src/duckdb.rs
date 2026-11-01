use crate::driver::{DatabaseDriver, DriverResult};
use crate::schema::{Column, DataType, DatabaseSchema, Index, Table};
use arrow::array::{Int64Builder, StringBuilder};
use arrow::datatypes::{DataType as ArrowDataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use duckdb::Connection;
use std::sync::{Arc, Mutex};

/// DuckDB and Parquet implementation of DatabaseDriver.
pub struct DuckDbDriver {
    path: Option<String>,
    conn: Option<Arc<Mutex<Connection>>>,
    is_parquet_mode: bool,
}

impl DuckDbDriver {
    pub fn new() -> Self {
        Self {
            path: None,
            conn: None,
            is_parquet_mode: false,
        }
    }

    pub fn with_connection(conn: Connection) -> Self {
        Self {
            path: Some(":memory:".to_string()),
            conn: Some(Arc::new(Mutex::new(conn))),
            is_parquet_mode: false,
        }
    }
}

impl Default for DuckDbDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DatabaseDriver for DuckDbDriver {
    async fn connect(&mut self, path: &str) -> DriverResult<()> {
        let lower = path.to_ascii_lowercase();
        let is_parquet = lower.ends_with(".parquet") || lower.ends_with(".pq");
        let is_csv = lower.ends_with(".csv") || lower.ends_with(".tsv");

        let conn = if path.is_empty() || path == ":memory:" || is_parquet || is_csv {
            Connection::open_in_memory()?
        } else {
            Connection::open(path)?
        };

        if is_parquet {
            // Register the parquet file as a view named `parquet_data`
            let escaped = path.replace('\'', "''");
            let view_sql = format!(
                "CREATE OR REPLACE VIEW parquet_data AS SELECT * FROM read_parquet('{}');",
                escaped
            );
            conn.execute_batch(&view_sql)?;
            self.is_parquet_mode = true;
        } else if is_csv {
            // Register the CSV/TSV file as a view named `csv_data`
            let escaped = path.replace('\'', "''");
            let view_sql = format!(
                "CREATE OR REPLACE VIEW csv_data AS SELECT * FROM read_csv_auto('{}');",
                escaped
            );
            conn.execute_batch(&view_sql)?;
            self.is_parquet_mode = true;
        } else {
            self.is_parquet_mode = false;
        }

        self.path = Some(path.to_string());
        self.conn = Some(Arc::new(Mutex::new(conn)));
        Ok(())
    }

    async fn introspect_schema(&self) -> DriverResult<DatabaseSchema> {
        let conn_arc = self
            .conn
            .as_ref()
            .ok_or_else(|| "Not connected to any DuckDB instance")?;
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        let mut tables = Vec::new();

        // 1. Fetch tables and views
        let query_tables = "SELECT table_name, table_type FROM information_schema.tables WHERE table_schema = 'main' ORDER BY table_name;";
        let mut tbl_stmt = conn.prepare(query_tables)?;
        let mut tbl_rows = tbl_stmt.query([])?;

        let mut table_names = Vec::new();
        while let Some(row) = tbl_rows.next()? {
            let name: String = row.get(0)?;
            table_names.push(name);
        }

        for tbl_name in table_names {
            let mut table = Table::new(&tbl_name);

            // 2. Fetch column details
            let query_cols = "SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_schema = 'main' AND table_name = ? ORDER BY ordinal_position;";
            let mut col_stmt = conn.prepare(query_cols)?;
            let mut col_rows = col_stmt.query([&tbl_name])?;

            while let Some(row) = col_rows.next()? {
                let name: String = row.get(0)?;
                let type_str: String = row.get(1)?;
                let is_nullable_str: String = row.get(2)?;
                let default_val: Option<String> = row.get(3)?;

                let data_type = DataType::from_sql(&type_str);
                let nullable = is_nullable_str.eq_ignore_ascii_case("YES");

                let mut col = Column::new(&name, data_type);
                if !nullable {
                    col = col.not_null();
                }
                if let Some(dv) = default_val {
                    col = col.default_val(dv);
                }
                table.columns.push(col);
            }

            // 3. Fetch indexes from duckdb_indexes()
            let query_idx = "SELECT index_name, is_unique, is_primary FROM duckdb_indexes() WHERE schema_name = 'main' AND table_name = ?;";
            if let Ok(mut idx_stmt) = conn.prepare(query_idx) {
                if let Ok(mut idx_rows) = idx_stmt.query([&tbl_name]) {
                    while let Ok(Some(row)) = idx_rows.next() {
                        let idx_name: String = row.get(0).unwrap_or_default();
                        let is_unique: bool = row.get(1).unwrap_or(false);
                        let is_primary: bool = row.get(2).unwrap_or(false);

                        let mut index = Index::new(&idx_name, &tbl_name, vec![], is_unique);
                        index.is_primary = is_primary;
                        table.indexes.push(index);
                    }
                }
            }

            tables.push(table);
        }

        Ok(DatabaseSchema::new(tables))
    }

    async fn execute_query(&self, sql: &str) -> DriverResult<Vec<RecordBatch>> {
        let conn_arc = self
            .conn
            .as_ref()
            .ok_or_else(|| "Not connected to any DuckDB database")?;
        let conn = conn_arc.lock().map_err(|e| e.to_string())?;

        let trimmed = sql.trim();
        let statements: Vec<&str> = trimmed
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if statements.is_empty() {
            return Ok(Vec::new());
        }

        if statements.len() > 1 {
            let prefix_sql = format!("{};", statements[..statements.len() - 1].join(";\n"));
            conn.execute_batch(&prefix_sql)?;
        }

        let final_stmt = statements[statements.len() - 1];
        let upper = final_stmt.to_uppercase();

        let is_select = upper.starts_with("SELECT")
            || upper.starts_with("DESCRIBE")
            || upper.starts_with("SHOW")
            || upper.starts_with("EXPLAIN")
            || upper.starts_with("WITH")
            || upper.starts_with("PRAGMA")
            || upper.starts_with("SUMMARIZE")
            || upper.starts_with("VALUES");

        if is_select {
            let mut stmt = conn.prepare(final_stmt)?;
            let arrow_stream = stmt.query_arrow([])?;
            let batches: Vec<RecordBatch> = arrow_stream.collect();
            Ok(batches)
        } else {
            // Execute DDL/DML statement
            let affected = conn.execute(final_stmt, [])?;

            let mut status_builder = StringBuilder::with_capacity(1, 16);
            status_builder.append_value("SUCCESS");

            let mut affected_builder = Int64Builder::with_capacity(1);
            affected_builder.append_value(affected as i64);

            let fields = vec![
                Field::new("status", ArrowDataType::Utf8, false),
                Field::new("rows_affected", ArrowDataType::Int64, false),
            ];
            let schema = Arc::new(Schema::new(fields));
            let batch = RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(status_builder.finish()),
                    Arc::new(affected_builder.finish()),
                ],
            )?;

            Ok(vec![batch])
        }
    }

    fn engine_name(&self) -> &'static str {
        if self.is_parquet_mode {
            "parquet"
        } else {
            "duckdb"
        }
    }

    fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    fn connection_path(&self) -> Option<&str> {
        self.path.as_deref()
    }
}
