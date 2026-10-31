pub mod diff;
pub mod driver;
pub mod duckdb;
pub mod schema;
pub mod sqlite;

pub use diff::{diff_schemas, ColumnDiff, SchemaDiff, SqlDialect, TableDiff};
pub use driver::{DatabaseDriver, DriverResult};
pub use duckdb::DuckDbDriver;
pub use schema::{Column, DataType, DatabaseSchema, ForeignKey, Index, Table};
pub use sqlite::SqliteDriver;
