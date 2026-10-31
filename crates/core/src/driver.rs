use crate::schema::DatabaseSchema;
use arrow::record_batch::RecordBatch;
use std::error::Error;

pub type DriverResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Unified asynchronous interface for database drivers.
#[async_trait::async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// Establish a connection to the database file or in-memory instance at the given path.
    async fn connect(&mut self, path: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Introspect and extract the canonical schema of the connected database.
    async fn introspect_schema(&self) -> Result<DatabaseSchema, Box<dyn Error + Send + Sync>>;

    /// Execute a SQL query or command and return the tabular output as Arrow RecordBatches.
    async fn execute_query(&self, sql: &str) -> Result<Vec<RecordBatch>, Box<dyn Error + Send + Sync>>;

    /// Return the friendly engine identifier (e.g., "sqlite", "duckdb", "parquet").
    fn engine_name(&self) -> &'static str {
        "unknown"
    }

    /// Check if the driver is currently connected.
    fn is_connected(&self) -> bool {
        true
    }

    /// Get the path of the currently connected database file if applicable.
    fn connection_path(&self) -> Option<&str> {
        None
    }
}
